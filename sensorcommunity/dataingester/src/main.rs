mod cache;
mod config;
mod http;
mod logging;
mod sensor_data;

use crate::config::{Arg, Command, Context, Manifest, crate_version, init_cli};
use axum::{
    BoxError, Router,
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri, uri::Authority},
    response::Redirect,
    routing::post,
};
use axum_extra::extract::Host;
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::cache::{CacheKey, load_cache};
use anyhow::Result;
use std::{
    collections::HashMap, future::Future, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration,
};
use tokio::signal;

pub const MANIFEST_NAME: &str = "dataingester.toml";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChipInfo {
    pub chip_id: String,
    pub city: String,
    pub info: String,
    pub lat: f64,
    pub lon: f64,
}

// chip_id,sensor_id,sensor_type
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SensorInfo {
    pub chip_id: String,
    pub sensor_id: String,
    pub sensor_type: String,
}

impl CacheKey for ChipInfo {
    fn id(&self) -> String {
        self.chip_id.clone()
    }
}

impl CacheKey for SensorInfo {
    fn id(&self) -> String {
        format!("{}:{}", self.chip_id, self.sensor_type)
    }
}

#[derive(Clone, Copy)]
struct Addresses {
    http_addr: SocketAddr,
    https_addr: SocketAddr,
}

pub struct SensorData<T> {
    json: T,
    sensor: String,
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
        .subcommand_required(true)
        .subcommand(clap::command!("serve"))
        .subcommand(
            clap::command!("import").arg(
                Arg::new("dir")
                    .short('d')
                    .long("dir")
                    .value_name("DIRECTORY")
                    .help("Import data from csv files in a folder and all subfolders.")
                    .value_parser(clap::value_parser!(std::path::PathBuf)),
            ),
        )
        .get_matches();

    let config_path = match matches.get_one::<String>("config") {
        Some(file) => file,
        _ => MANIFEST_NAME,
    };

    let (config, log_guard, ctx) = init_cli(config_path).unwrap();

    match matches.subcommand() {
        Some(("serve", _matches)) => {
            serve(config, log_guard, ctx).await;
        }
        Some(("import", matches)) => {
            let dir = match matches.get_one::<std::path::PathBuf>("dir") {
                Some(d) => d,
                _ => {
                    tracing::error!("invalid import directory");
                    return;
                }
            };
            import(config, log_guard, ctx, dir).await;
        }
        _ => unreachable!("clap should ensure we don't get here"),
    };
}

fn get_writers(config: &Manifest) -> Vec<Arc<dyn crate::sensor_data::DataWriter>> {
    // register writers
    let mut writers = vec![];
    if config.influxdb.url.len() > 0 {
        let inf: Arc<dyn crate::sensor_data::DataWriter> = Arc::new(
            crate::sensor_data::InfluxDB2DataWriter::new(config.influxdb.clone()),
        );
        writers.push(inf);
    }
    if config.influxdb3.url.len() > 0 {
        let inf: Arc<dyn crate::sensor_data::DataWriter> = Arc::new(
            crate::sensor_data::InfluxDB3DataWriter::new(config.influxdb3.clone()),
        );
        writers.push(inf);
    }
    if config.questdb.addr.len() > 0 {
        let inf: Arc<dyn crate::sensor_data::DataWriter> = Arc::new(
            crate::sensor_data::QuestDBDataWriter::new(config.questdb.clone()),
        );
        writers.push(inf);
    }
    writers
}

async fn serve(
    config: Manifest,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
    ctx: Context,
) {
    tracing::debug!("working directory: {}", ctx.working_dir.display());

    let chips_filepath = shellexpand::env(&config.chips_filepath.as_os_str().to_string_lossy())
        .unwrap()
        .as_ref()
        .to_owned();

    // load chip cache with hot reload
    let (chip_cache, _watcher) = match load_cache::<ChipInfo>(&chips_filepath) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("could not load the chip info cache: {}", e);
            return;
        }
    };

    let sensors_filepath = shellexpand::env(&config.sensors_filepath.as_os_str().to_string_lossy())
        .unwrap()
        .as_ref()
        .to_owned();

    // load chip cache with hot reload
    let (sensor_cache, _watcher) = match load_cache::<SensorInfo>(&sensors_filepath) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("could not load the chip info cache: {}", e);
            return;
        }
    };

    let http_addr: SocketAddr = config.http_addr.trim().parse().unwrap();

    //Create a handle for our TLS server so the shutdown signal can all shutdown
    let handle = axum_server::Handle::new();
    //save the future for easy shutting down of redirect server
    let shutdown_future = shutdown_signal(handle.clone());

    let sensor_data_dir = PathBuf::from(
        shellexpand::env(&config.sensor_data_dir.as_os_str().to_string_lossy())
            .unwrap()
            .as_ref(),
    );

    // register writers
    let writers = get_writers(&config);

    let mut logins: HashMap<String, String> = HashMap::new();
    for login in config.logins {
        logins.insert(login.username.to_lowercase(), login.password);
    }

    // let use_influxdb_3 = influxdb_settings.url.len() == 0;

    let app = Router::new()
        .route("/write", post(http::handler))
        .with_state(http::ReqState {
            chip_cache: chip_cache,
            sensor_cache: sensor_cache,
            sensor_data_dir,
            measure_name_to_field: config.measure_name_to_field,
            measure_name_to_sensor_type: config.measure_name_to_sensor_type,
            writers,
            logins,
        });
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

async fn import(
    config: Manifest,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
    _ctx: Context,
    dir: &PathBuf,
) {

    // register writers
    let writers = get_writers(&config);


    let sensors_filepath = shellexpand::env(&config.sensors_filepath.as_os_str().to_string_lossy())
        .unwrap()
        .as_ref()
        .to_owned();

    // load chip cache with hot reload
    let (sensor_cache, _watcher) = match load_cache::<SensorInfo>(&sensors_filepath) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("could not load the chip info cache: {}", e);
            return;
        }
    };
    // Walk through directory and all subdirectories
    for entry in WalkDir::new(dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Check if the file is a CSV
        if path.is_file() && path.extension().map_or(false, |ext| ext == "csv") {
            match sensor_data::import_csv(path, &config, &writers, sensor_cache.clone()).await {
                Ok(r) => {
                    if r.record_count > 0 {
                        tracing::info!(
                            "Successfully imported {} values from: {}",
                            r.record_count,
                            path.display()
                        );
                    }
                }
                Err(e) => tracing::error!("Error loading CSV {}: {}", path.display(), e),
            }
        }
    }
}
