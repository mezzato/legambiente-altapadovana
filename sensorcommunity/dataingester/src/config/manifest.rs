use crate::config::hostname;
use crate::logging;
use anyhow::Context;
use chrono::prelude::*;
use digest::Digest;
use serde::{Deserialize, Serialize};
use sha2;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, reload};

static DEFAULT_LOG_FILE_PATH: &str = "$HOME/.config/dataingester/log.txt";
static DEFAULT_PERF_ADDR: &str = "0.0.0.0:4000";
static DEFAULT_HTTP_ADDR: &str = "0.0.0.0:7878";
static DEFAULT_HTTPS_ADDR: &str = "0.0.0.0:3878";

// Toy example, do not use it in practice!
// Instead use crates from: https://github.com/RustCrypto/password-hashing
fn hash_password<D: Digest>(password: &str, salt: &str, output: &mut [u8]) {
    let mut hasher = D::new();
    hasher.update(password.as_bytes());
    hasher.update(b"$");
    hasher.update(salt.as_bytes());
    output.copy_from_slice(hasher.finalize().as_slice())
}

// ManifestName is the manifest file name used by dep.
// pub const MANIFEST_NAME: &str = "config.toml";

// Manifest holds manifest file data and implements gps.RootManifest.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Manifest {
    #[serde(skip_serializing, skip_deserializing)]
    pub path: PathBuf,
    #[serde(default)]
    pub tls_dir: PathBuf,
    #[serde(default)]
    pub chips_filepath: PathBuf,
    #[serde(default)]
    pub sensors_filepath: PathBuf,
    #[serde(default)]
    pub sensor_data_dir: PathBuf,
    pub perf: PerfConfig,
    pub logging: Logging,
    #[serde(default)]
    pub http_addr: String,
    #[serde(default)]
    pub https_addr: String,
    pub influxdb: InfluxDB,
    pub influxdb3: InfluxDB3,
    pub questdb: QuestDB,
    pub logins: Vec<Login>,
    pub measure_name_to_field: HashMap<String, String>,
    pub measure_name_to_sensor_type: HashMap<String, String>,
}

impl Manifest {
    pub fn from_default(
        path: &str,
    ) -> std::result::Result<(Manifest, tracing_appender::non_blocking::WorkerGuard), anyhow::Error>
    {
        let mut manifest: Manifest = Default::default();
        manifest.path = PathBuf::from(path);
        manifest.tls_dir = PathBuf::from("tls");
        manifest.chips_filepath = PathBuf::from("chips.csv");
        manifest.sensors_filepath = PathBuf::from("sensors.csv");
        manifest.sensor_data_dir = PathBuf::from("./chip_data");
        // manifest.sensors_filepath = PathBuf::from("sensors.csv");

        manifest.http_addr = DEFAULT_HTTP_ADDR.to_string();
        manifest.https_addr = DEFAULT_HTTPS_ADDR.to_string();

        let mut buf = [0u8; 32];
        // Create a normal DateTime from the NaiveDateTime
        let datetime = Utc::now().format("%Y-%m-%d %H:%M:%S");

        // Get an RNG:

        use rand::distr::{Alphanumeric, SampleString};
        let salt = Alphanumeric.sample_string(&mut rand::rng(), 16);
        let salt = salt + &hostname::gethostname().to_string_lossy();

        hash_password::<sha2::Sha256>(&datetime.to_string(), &salt, &mut buf);

        let hex: String = buf
            .iter()
            .map(|b| format!("{:x}", b).to_string())
            .collect::<Vec<String>>()
            .join("");

        manifest.logins.push(Login {
            username: "myuserA".to_owned(),
            password: hex.clone() + "_A",
        });
        manifest.logins.push(Login {
            username: "myuserB".to_owned(),
            password: hex + "_B",
        });

        let hash = &mut manifest.measure_name_to_field;
        hash.insert("SDS_P1".to_owned(), "P1".to_owned());
        hash.insert("SDS_P2".to_owned(), "P2".to_owned());
        hash.insert("temperature".to_owned(), "temperature".to_owned());
        hash.insert("humidity".to_owned(), "humidity".to_owned());
        hash.insert("BMP_temperature".to_owned(), "temperature".to_owned());
        hash.insert("BMP_pressure".to_owned(), "pressure".to_owned());
        hash.insert("BME280_temperature".to_owned(), "temperature".to_owned());
        hash.insert("BME280_humidity".to_owned(), "humidity".to_owned());
        hash.insert("BME280_pressure".to_owned(), "pressure".to_owned());

        let hash = &mut manifest.measure_name_to_sensor_type;
        hash.insert("SDS_P1".to_owned(), "SDS011".to_owned());
        hash.insert("SDS_P2".to_owned(), "SDS011".to_owned());
        hash.insert("temperature".to_owned(), "DHT22".to_owned());
        hash.insert("humidity".to_owned(), "DHT22".to_owned());
        hash.insert("BMP_temperature".to_owned(), "BMP180".to_owned());
        hash.insert("BMP_pressure".to_owned(), "BMP180".to_owned());
        hash.insert("BME280_temperature".to_owned(), "BME280".to_owned());
        hash.insert("BME280_humidity".to_owned(), "BME280".to_owned());
        hash.insert("BME280_pressure".to_owned(), "BME280".to_owned());

        let guard = manifest.logging.setup()?;
        Ok((manifest, guard))
    }

    pub fn load(
        path: &str,
    ) -> std::result::Result<(Manifest, tracing_appender::non_blocking::WorkerGuard), anyhow::Error>
    {
        let toml_str = &fs::read_to_string(path)?;
        let mut manifest = toml::from_str::<Manifest>(toml_str)
            .with_context(|| ("Unable to load manifest: {}"))?;
        manifest.path = PathBuf::from(path);
        let guard = manifest.logging.setup()?;
        Ok((manifest, guard))
    }

    pub fn save(&self) -> std::result::Result<(), anyhow::Error> {
        let toml = toml::to_string(&self)?;
        fs::write(self.path.to_str().unwrap(), toml)?;
        Ok(())
    }

    pub fn change_log_level(&mut self, level: &str) -> std::result::Result<(), anyhow::Error> {
        if let Some(reload_fn) = &mut self.logging.reload_fn.0 {
            reload_fn(level)?;
            self.logging.level = level.to_lowercase().to_string();
            self.save()?;
        }

        Ok(())
    }
}

pub struct ReloadFn(Option<Box<dyn Fn(&str) -> std::result::Result<(), anyhow::Error>>>);

impl Default for ReloadFn {
    fn default() -> Self {
        ReloadFn(None)
    }
}

unsafe impl Sync for ReloadFn {}
unsafe impl Send for ReloadFn {}

impl std::fmt::Debug for ReloadFn {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0 {
            None => write!(f, "({})", "None"),
            Some(_) => write!(f, "({})", "Some"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Logging {
    #[serde(rename = "logtostderr", default)]
    pub log_to_stderr: bool, //   `toml:"logtostderr"`
    pub filename: PathBuf, //`toml:"filename"`
    #[serde(rename = "max-size-mb", default)]
    pub max_size_mb: u32, //`toml:"max-size"`
    #[serde(rename = "max-backups", default)]
    pub max_backups: u32, //`toml:"max-backups"`
    #[serde(rename = "max-age-days", default)]
    pub max_age_days: u32, //`toml:"max-age"`
    #[serde(default)]
    pub compress: bool, //`toml:"compress"`
    #[serde(rename = "level", default)] //  = Some("localhost:1883")
    pub level: String, // `toml:"mqtt-addr"`

    #[serde(skip_serializing, skip_deserializing)]
    reload_fn: ReloadFn,
}

impl Default for Logging {
    fn default() -> Self {
        Logging {
            log_to_stderr: true,
            filename: PathBuf::from_str(DEFAULT_LOG_FILE_PATH).unwrap(),
            max_size_mb: 10,
            max_backups: 10,
            max_age_days: 30,
            compress: false,
            level: "ERROR".to_string(),
            reload_fn: ReloadFn(None),
        }
    }
}

impl Logging {
    fn setup(
        &mut self,
    ) -> std::result::Result<tracing_appender::non_blocking::WorkerGuard, anyhow::Error> {
        let log_to_stderr = self.log_to_stderr;
        let log_file_path;

        let (non_blocking_writer, guard) = match log_to_stderr {
            true => tracing_appender::non_blocking(std::io::stderr()),
            false => {
                /*
                max_size_mb: 10,
                max_backups: 10,
                max_age_days: 30,
                compress: false,
                */
                let log_path = self.filename.to_string_lossy();
                log_file_path = PathBuf::from(shellexpand::env(&log_path)?.as_ref());

                // expand the file path and replace existing
                self.filename = log_file_path;

                let rotated_file = logging::FileRotate::new(
                    self.filename.clone(),
                    logging::CountSuffix::new(self.max_backups as usize),
                    logging::ContentLimit::Bytes(self.max_size_mb as usize * 1024 * 1024),
                    logging::Compression::None,
                );

                tracing_appender::non_blocking(rotated_file)
            }
        };

        // filter by level

        let filtered_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&self.level.to_lowercase()))?;

        let (filtered_layer, reload_handle) = reload::Layer::new(filtered_layer);

        let filtered_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking_writer)
            .with_target(false)
            .with_ansi(false)
            .with_filter(filtered_layer);

        // let console_layer = console_subscriber::spawn();

        tracing_subscriber::registry()
            // add the console layer to the subscriber
            // .with(console_layer)
            // add other layers...
            .with(filtered_layer)
            // .with(...)
            .try_init()
            .with_context(|| ("Unable to set global default subscriber: {}"))?;

        let reload_fn = move |level: &str| -> std::result::Result<(), anyhow::Error> {
            let new_filter = EnvFilter::try_new(level.to_lowercase())?;
            reload_handle.modify(|filter| *filter = new_filter)?;
            Ok(())
        };

        self.reload_fn = ReloadFn(Some(Box::new(reload_fn)));

        Ok(guard)
    }
}

impl Default for PerfConfig {
    fn default() -> Self {
        PerfConfig {
            enabled: false,
            perf_addr: String::from(DEFAULT_PERF_ADDR),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PerfConfig {
    #[serde(rename = "enabled", default)]
    pub enabled: bool,
    #[serde(rename = "perf-addr", default)]
    pub perf_addr: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Login {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

impl Default for InfluxDB {
    fn default() -> Self {
        InfluxDB {
            url: "".to_owned(),
            token: "".to_owned(),
            org: "".to_owned(),
            bucket: "mypassword".to_owned(),
            measurement: "".to_owned(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InfluxDB {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub org: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub measurement: String,
}

impl Default for InfluxDB3 {
    fn default() -> Self {
        InfluxDB3 {
            url: "".to_owned(),
            token: "".to_owned(),
            database: "mydb".to_owned(),
            table: "".to_owned(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InfluxDB3 {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub table: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QuestDB {
    #[serde(default)]
    pub use_https: bool,
    #[serde(default)]
    pub addr: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub table: String,
}

impl Default for QuestDB {
    fn default() -> Self {
        QuestDB {
            use_https: true,
            addr: "".to_owned(),
            username: "".to_owned(),
            password: "".to_owned(),
            table: "".to_owned(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_serialize() {
        let toml_str = r#"
[logging]
logtostderr = true
filename = "$HOME/.config/dataingester/log.txt"
max-size-mb = 10
max-backups = 10
max-age-days = 30
compress = false
level = "DEBUG"
		"#;

        let decoded: Manifest = toml::from_str(toml_str).unwrap();
        println!("{:#?}", decoded);
        // assert!(decoded == toml_str);
    }
}
