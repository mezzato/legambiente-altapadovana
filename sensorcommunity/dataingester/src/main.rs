mod config;
mod logging;

use crate::config::{crate_version, init_cli, Arg, Command};
use axum::{
    body::{Body, Bytes},
    extract::{FromRequest, FromRequestParts, Request},
    handler::HandlerWithoutStateExt,
    http::{request::Parts, uri::Authority, HeaderMap, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::post,
    BoxError, Json, RequestExt, RequestPartsExt, Router,
};
use axum_extra::extract::Host;
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use axum_server::tls_rustls::RustlsConfig;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Display;
use std::{future::Future, net::SocketAddr, path::PathBuf, time::Duration};
use tokio::signal;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub const MANIFEST_NAME: &str = "dataingester.toml";

#[derive(Clone, Copy)]
struct Addresses {
    http_addr: SocketAddr,
    https_addr: SocketAddr,
}

#[tokio::main]
async fn main() {
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

    let http_addr: SocketAddr = config.http_addr.trim().parse().unwrap();

    //Create a handle for our TLS server so the shutdown signal can all shutdown
    let handle = axum_server::Handle::new();
    //save the future for easy shutting down of redirect server
    let shutdown_future = shutdown_signal(handle.clone());

    let app = Router::new().route("/write", post(handler));
    //.layer(middleware::from_fn(print_request_body));

    let https_addr = config.https_addr.trim();
    if https_addr.is_empty() {
        let addr = SocketAddr::from(http_addr);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        tracing::info!("listening on address: {}", addr);

        // Run the server with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_future)
            .await
            .unwrap();
    } else {


        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install default CryptoProvider");
        
        let https_addr: SocketAddr = config.https_addr.parse().unwrap();
        let addresses = Addresses {
            http_addr,
            https_addr,
        };
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


        // run https server
        tracing::debug!("listening on TLS address: {}", addresses.https_addr);
        axum_server::bind_rustls(addresses.https_addr, config)
            .handle(handle)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
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

#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    // software_version: String,
}

async fn handler(SensorData { json, sensor }: SensorData<Payload>) -> &'static str {
    tracing::debug!(?sensor, "sensor");
    tracing::debug!(?json, "json body");
    println!("---- HIT ----");
    "Hello, World!"
}

// extractor that shows how to consume the request body upfront
// struct BufferRequestBody(Bytes);

const X_SENSOR_HEADER: &str = "x-sensor";

struct SensorData<T> {
    json: T,
    sensor: String,
}

impl<S, T> FromRequest<S> for SensorData<T>
where
    S: Send + Sync,
    Json<T>: FromRequest<()>,
    T: 'static,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        tracing::debug!(request = ?req);

        // Extract the token from the authorization header
        let sensor_header = req.headers().get(X_SENSOR_HEADER);
        let sensor = sensor_header.and_then(|value| value.to_str().ok());
        let sensor = sensor.unwrap_or_default().to_owned();

        let Json(json) = req.extract().await.map_err(IntoResponse::into_response)?;

        let data = SensorData { json, sensor };

        Ok(data)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    company: String,
    exp: usize,
}

#[derive(Debug, Serialize)]
struct AuthBody {
    access_token: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct AuthPayload {
    client_id: String,
    client_secret: String,
}

#[derive(Debug)]
enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
}

async fn redirect_http_to_https<F>(addrs: Addresses, signal: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    fn make_https(host: &str, uri: Uri, https_port: u16) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/write".parse().unwrap());
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
    tracing::debug!(
        "listening on address: {}, redirecting to address: {}",
        addr,
        addrs.https_addr
    );
    axum::serve(listener, redirect.into_make_service())
        .with_graceful_shutdown(signal)
        .await
        .unwrap();
}
