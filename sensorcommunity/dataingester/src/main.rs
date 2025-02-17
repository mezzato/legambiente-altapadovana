mod config;
mod logging;

use crate::config::{crate_version, init_cli, Arg, Command};
use axum::{
    body::{Body, Bytes},
    extract::{FromRequest, Request},
    handler::HandlerWithoutStateExt,
    http::{request::Parts, uri::Authority, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::post,
    BoxError, Router,
};
use axum_extra::extract::Host;
use axum_server::tls_rustls::RustlsConfig;
use http_body_util::BodyExt;
use std::{future::Future, net::SocketAddr, path::PathBuf, time::Duration};
use tokio::signal;

pub const MANIFEST_NAME: &str = "dataingester.toml";

#[derive(Clone, Copy)]
struct Addresses {
    http_addr: SocketAddr,
    https_addr: SocketAddr,
}

#[tokio::main]
async fn main() {
    /*
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    */

    let matches = Command::new("dataingester")
        .version(crate_version!())
        .author("Legambiente")
        .about("dataingester ingests data from Sensor Community nodes")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file path")
                .default_value(MANIFEST_NAME),
        )
        .arg(
            Arg::new("reset_auth_key")
                .long("reset_auth_key")
                .help("Resets the authorization token"),
        )
        .get_matches();

    let config_path = match matches.get_one::<String>("config") {
        Some(file) => file,
        _ => MANIFEST_NAME,
    };

    let (config, _log_guard, ctx) = init_cli(config_path).unwrap();

    tracing::debug!("working directory: {}", ctx.working_dir.display());

    let http_addr: SocketAddr = config.http_addr.parse().unwrap();
    let https_addr: SocketAddr = config.https_addr.parse().unwrap();

    let addresses = Addresses {
        http_addr,
        https_addr,
    };

    //Create a handle for our TLS server so the shutdown signal can all shutdown
    let handle = axum_server::Handle::new();
    //save the future for easy shutting down of redirect server
    let shutdown_future = shutdown_signal(handle.clone());

    // optional: spawn a second server to redirect http requests to this server
    tokio::spawn(redirect_http_to_https(addresses, shutdown_future));

    let tls_dir = PathBuf::from(
        shellexpand::env(&config.tls_dir.as_os_str().to_string_lossy())
            .unwrap()
            .as_ref(),
    );

    const CERT_FILE: &str = "cert.pem";
    const KEY_FILE: &str = "key.pem";

    // configure certificate and private key used by https
    let config = RustlsConfig::from_pem_file(tls_dir.join(CERT_FILE), tls_dir.join(KEY_FILE))
        .await
        .map_err(|e| {
            format!(
            "error loading TLS config files from folder: {}, certificate: {}, private key: {}, error: {}",
            &tls_dir.display(),
            CERT_FILE, KEY_FILE,
            e
        )
        })
        .unwrap();

    let app = Router::new()
        .route("/digest", post(handler))
        .layer(middleware::from_fn(print_request_body));

    // run https server
    tracing::debug!("listening on TLS address: {}", addresses.https_addr);
    axum_server::bind_rustls(addresses.https_addr, config)
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn shutdown_signal(handle: axum_server::Handle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Received termination signal shutting down");
    handle.graceful_shutdown(Some(Duration::from_secs(10))); // 10 secs is how long docker will wait
                                                             // to force shutdown
}

// middleware that shows how to consume the request body upfront
async fn print_request_body(request: Request, next: Next) -> Result<impl IntoResponse, Response> {
    let request = buffer_request_body(request).await?;

    Ok(next.run(request).await)
}

// the trick is to take the request apart, buffer the body, do what you need to do, then put
// the request back together
async fn buffer_request_body(request: Request) -> Result<Request, Response> {
    let (parts, body) = request.into_parts();

    // this won't work if the body is an long running stream
    let bytes = body
        .collect()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?
        .to_bytes();

    do_thing_with_request_body(bytes.clone(), parts.clone());

    Ok(Request::from_parts(parts, Body::from(bytes)))
}

fn do_thing_with_request_body(bytes: Bytes, parts: Parts) {
    tracing::debug!(body = ?bytes);
    tracing::debug!(headers = ?parts.headers);
}

async fn handler(BufferRequestBody(body): BufferRequestBody) {
    tracing::debug!(?body, "handler received body");
}

// extractor that shows how to consume the request body upfront
struct BufferRequestBody(Bytes);

// we must implement `FromRequest` (and not `FromRequestParts`) to consume the body
impl<S> FromRequest<S> for BufferRequestBody
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let body = Bytes::from_request(req, state)
            .await
            .map_err(|err| err.into_response())?;

        // do_thing_with_request_body(body.clone());

        Ok(Self(body))
    }
}

async fn redirect_http_to_https<F>(addrs: Addresses, signal: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    fn make_https(host: &str, uri: Uri, https_port: u16) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let authority: Authority = host.parse()?;
        let bare_host = match authority.port() {
            Some(port_struct) => authority
                .as_str()
                .strip_suffix(port_struct.as_str())
                .unwrap()
                .strip_suffix(':')
                .unwrap(), // if authority.port() is Some(port) then we can be sure authority ends with :{port}
            None => authority.as_str(),
        };

        parts.authority = Some(format!("{bare_host}:{https_port}").parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(&host, uri, addrs.https_addr.port()) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(addrs.http_addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on address: {addr}");
    axum::serve(listener, redirect.into_make_service())
        .with_graceful_shutdown(signal)
        .await
        .unwrap();
}
