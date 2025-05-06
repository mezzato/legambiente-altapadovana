mod import_csv;
mod influxdb2;
mod influxdb3;
mod questdb;
mod sensor_data;
use async_trait::async_trait;

use serde::{Deserialize, Serialize};
pub use {
    import_csv::import_csv, influxdb2::InfluxDB2DataWriter, influxdb3::InfluxDB3DataWriter,
    questdb::QuestDBDataWriter, sensor_data::*,
};

pub const CHIP_ID: &str = "chip_id";
pub const SENSOR_ID: &str = "sensor_id";
pub const SENSOR_TYPE: &str = "sensor_type";
pub const LAT: &str = "lat";
pub const LON: &str = "lon";
pub const CITY: &str = "city";
pub const INFO: &str = "info";

//const TIMESTAMP: &str = "timestamp";
pub const P1: &str = "P1";
pub const SDS_P1: &str = "SDS_P1";
pub const DUR_P1: &str = "durP1";
pub const RATIO_P1: &str = "ratioP1";
pub const P2: &str = "P2";

pub const SDS_P2: &str = "SDS_P2";
pub const DUR_P2: &str = "durP2";
pub const RATIO_P2: &str = "ratioP2";
pub const TEMPERATURE: &str = "temperature";
pub const BMP_TEMPERATURE: &str = "BMP_temperature";
pub const BME280_TEMPERATURE: &str = "BMP280_temperature";
pub const HUMIDITY: &str = "humidity";
pub const BMP_PRESSURE: &str = "BMP_pressure";
pub const BME280_HUMIDITY: &str = "BME280_humidity";
pub const BME280_PRESSURE: &str = "BME280_pressure";

pub const SIGNAL: &str = "signal";
pub const TIMESTAMP: &str = "timestamp";

pub const FIELD: &str = "field";
pub const VALUE: &str = "value";

// Note that structs can derive both Serialize and Deserialize!
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DataRecord<'a> {
    pub chip_id: &'a str,
    pub lat: f64,
    pub lon: f64,
    pub timestamp: i64,
    #[serde(rename = "P1")]
    pub p1: Option<f64>,
    #[serde(rename = "ratioP1")]
    pub ratio_p1: Option<f64>,
    #[serde(rename = "durP1")]
    pub dur_p1: Option<i64>,
    #[serde(rename = "P2")]
    pub p2: Option<f64>,
    #[serde(rename = "ratioP2")]
    pub ratio_p2: Option<f64>,
    #[serde(rename = "durP2")]
    pub dur_p2: Option<i64>,
    #[serde(rename = "SDS_P1")]
    pub sds_p1: Option<f64>,
    #[serde(rename = "SDS_P2")]
    pub sds_p2: Option<f64>,
    pub temperature: Option<f64>,
    pub humidity: Option<f64>,
    #[serde(rename = "BMP_temperature")]
    pub bmp_temperature: Option<f64>,
    #[serde(rename = "BMP_pressure")]
    pub bmp_pressure: Option<f64>,
    #[serde(rename = "BME280_temperature")]
    pub bmp280_temperature: Option<f64>,
    #[serde(rename = "BMP280_humidity")]
    pub bmp280_humidity: Option<f64>,
    #[serde(rename = "BMP280_pressure")]
    pub bmp280_pressure: Option<f64>,
    pub signal: Option<i64>,
    pub city: String,
    pub info: String,
}

pub struct RecordValue {
    sensor_id: String,
    sensor_type: String,
    field: String,
    value: f64,
}

pub struct Record {
    pub chip_id: String,
    pub lat: f64,
    pub lon: f64,
    pub city: String,
    pub info: String,
    pub values: Vec<RecordValue>,
    pub timestamp: u128,
}

#[async_trait]
pub trait DataWriter: Sync + Send {
    async fn write(&self, recs: &[Record]) -> anyhow::Result<()>;
}
