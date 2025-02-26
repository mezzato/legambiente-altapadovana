mod config;
mod logging;

use crate::config::{crate_version, init_cli, Arg, Command};
use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    handler::HandlerWithoutStateExt,
    http::{uri::Authority, StatusCode, Uri},
    response::Redirect,
    routing::post,
    BoxError, Json, RequestPartsExt, Router,
};
use axum_extra::extract::Host;
use axum_server::tls_rustls::RustlsConfig;

use serde::{Deserialize, Serialize};
use serde_json::json;
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

// {"esp8266id": "15303512", "software_version": "NRZ-2024-135", "sensordatavalues":[{"value_type":"SDS_P1","value":"67.22"},{"value_type":"SDS_P2","value":"34.47"},{"value_type":"temperature","value":"2.00"},{"value_type":"humidity","value":"38.40"},{"value_type":"samples","value":"5403023"},{"value_type":"min_micro","value":"25"},{"value_type":"max_micro","value":"73179"},{"value_type":"interval","value":"145000"},{"value_type":"signal","value":"-70"}]}

#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    software_version: String,
    sensordatavalues: Vec<SensorValue>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SensorValue {
    value_type: String,
    value: String,
}

async fn handler(SensorData { json, sensor }: SensorData<Payload>) {
    // tracing::debug!(?sensor, "sensor");
    // tracing::debug!(?json, "json body");
    println!("sensor: {}, json: {:?}", sensor, json);
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
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    T: 'static,
{
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // tracing::debug!(request = ?req);

        // Extract the token from the authorization header
        let sensor_header = req.headers().get(X_SENSOR_HEADER);
        let sensor = sensor_header.and_then(|value| value.to_str().ok());
        let sensor = sensor.unwrap_or_default().to_owned();
        /*
        let Json(json) = req.extract().await.map_err(|err| {
            let resp = IntoResponse::into_response(err);
            println!("{:?}", resp);
            resp
        })?;
        */

        let (mut parts, body) = req.into_parts();

        tracing::debug!(headers = ?parts.headers);
        // tracing::debug!(body = ?body);

        // We can use other extractors to provide better rejection messages.
        // For example, here we are using `axum::extract::MatchedPath` to
        // provide a better error message.
        //
        // Have to run that first since `Json` extraction consumes the request.
        let path = parts
            .extract::<axum::extract::MatchedPath>()
            .await
            .map(|path| path.as_str().to_owned())
            .ok();

        let req = Request::from_parts(parts, body);

        let json = match axum::Json::<T>::from_request(req, state).await {
            Ok(value) => Ok(value.0),
            // convert the error from `axum::Json` into whatever we want
            Err(rejection) => {
                // println!("--- rejection: {}", rejection.body_text());
                let payload = json!({
                    "message": rejection.body_text(),
                    "origin": "custom_extractor",
                    "path": path,
                });

                Err((rejection.status(), axum::Json(payload)))
            }
        }?;

        let data = SensorData { json, sensor };

        Ok(data)
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
