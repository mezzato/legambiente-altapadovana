use std::{collections::HashMap, fs::OpenOptions, io::Seek};

use crate::cache::Cache;
use anyhow::{Result, anyhow};
use chrono::Utc;
use influxdb::InfluxDbWriteable;
use serde::{Deserialize, Serialize};

use super::{
    BME280_HUMIDITY, BME280_PRESSURE, BME280_TEMPERATURE, BMP_PRESSURE, BMP_TEMPERATURE, CHIP_ID,
    CITY, DUR_P1, DUR_P2, HUMIDITY, INFO, LAT, LON, P1, P2, RATIO_P1, RATIO_P2, SDS_P1, SDS_P2,
    SENSOR_ID, SENSOR_TYPE, SIGNAL, TEMPERATURE,
};

// "Time", durP1;ratioP1;P1;durP2;ratioP2;P2;SDS_P1;SDS_P2;Temp;Humidity;BMP_temperature;BMP_pressure;BME280_temperature;BME280_humidity;BME280_pressure;Samples;Min_cycle;Max_cycle;Signal\n"
// chip_id;lat;lon;timestamp;P1;durP1;ratioP1;P2;durP2;ratioP2;temperature;humidity;pressure;signal

/*
#[derive(InfluxDbWriteable)]
pub struct InfluxDataRecord<'a> {
    chip_id: &'a str,
    lat: f64,
    lon: f64,
    time: DateTime<Utc>,

    city: String,
    description: String,
}
*/

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    software_version: String,
    sensordatavalues: Vec<SensorValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SensorValue {
    value_type: String,
    value: String,
}

pub fn get_sensor_id(
    sensor_cache: &Cache<crate::SensorInfo>,
    chip_id: &str,
    sensor_type: &str,
) -> Result<String, anyhow::Error> {
    let cache_id = format!("{}:{}", chip_id, sensor_type);
    match sensor_cache.read() {
        Ok(cache) => {
            if let Some(info) = cache.get(&cache_id) {
                Ok(info.sensor_id.to_owned())
            } else {
                Err(anyhow!("missing sensory id for key: {}", cache_id))
            }
        }
        Err(e) => Err(anyhow!("{}", e)),
    }
}

pub async fn write(
    influxdb_settings: &crate::config::InfluxDB,
    influxdb3_settings: &crate::config::InfluxDB3,
    file_path: &std::path::PathBuf,
    measure_name_to_field: &HashMap<String, String>,
    measure_name_to_sensor_type: &HashMap<String, String>,
    chip_cache: Cache<crate::ChipInfo>,
    sensor_cache: Cache<crate::SensorInfo>,
    chip_id: &str,
    payload: Payload,
) -> Result<(), Box<dyn std::error::Error>> {
    // let mut wtr = csv::Writer::from_path(file_path)?;

    let timestamp = Utc::now().timestamp();

    let mut d = crate::sensor_data::DataRecord::default();

    match chip_cache.read() {
        Ok(cache) => {
            if let Some(info) = cache.get(chip_id) {
                d.city = info.city.to_owned();
                d.info = info.info.to_owned();
                d.lat = info.lat;
                d.lon = info.lon;
            } else {
                tracing::error!(
                    "skipping missing chip id: {}. If you want to record its data add it to the chip file.",
                    chip_id,
                );
                return Ok(());
            }
        }
        _ => {
            tracing::error!(
                "skipping chip id: {}. Error trying to acquire cache lock.",
                chip_id,
            );
            return Ok(());
        }
    }

    d.timestamp = timestamp;
    d.chip_id = chip_id;

    let mut points = vec![];

    let mut write_queries = Vec::<influxdb::WriteQuery>::new();

    let use_influxdb_3 = influxdb_settings.url.len() == 0;

    for data_row in payload.sensordatavalues {
        // write csv first
        match data_row.value_type.as_str() {
            P1 => {
                let v = data_row.value.parse::<f64>();
                d.p1 = v.clone().ok();
                // dp.field(P1, v.unwrap_or_default() as f64)
            }
            SDS_P1 => {
                let v = data_row.value.parse::<f64>();
                d.sds_p1 = v.clone().ok();
                // dp.field(P1, v.unwrap_or_default() as f64)
            }
            DUR_P1 => {
                let v = data_row.value.parse::<i64>();
                d.dur_p1 = v.clone().ok();
                // dp.field(DUR_P1, v.unwrap_or_default() as i64)
            }
            RATIO_P1 => {
                let v = data_row.value.parse::<f64>();
                d.ratio_p1 = v.clone().ok();
                // dp.field(RATIO_P1, v.unwrap_or_default() as f64)
            }
            P2 => {
                let v = data_row.value.parse::<f64>();
                d.p2 = v.clone().ok();
                // dp.field(P2, v.unwrap_or_default() as f64)
            }
            SDS_P2 => {
                let v = data_row.value.parse::<f64>();
                d.sds_p2 = v.clone().ok();
                // dp.field(P1, v.unwrap_or_default() as f64)
            }
            DUR_P2 => {
                let v = data_row.value.parse::<i64>();
                d.dur_p2 = v.clone().ok();
                // dp.field(DUR_P2, v.unwrap_or_default() as i64)
            }
            RATIO_P2 => {
                let v = data_row.value.parse::<f64>();
                d.ratio_p2 = v.clone().ok();
                // dp.field(RATIO_P2, v.unwrap_or_default() as f64)
            }
            TEMPERATURE => {
                let v = data_row.value.parse::<f64>();
                d.temperature = v.clone().ok();
                // dp.field(TEMPERATURE, v.unwrap_or_default() as f64)
            }
            BMP_TEMPERATURE => {
                let v = data_row.value.parse::<f64>();
                d.bmp_temperature = v.clone().ok();
                // dp.field(TEMPERATURE, v.unwrap_or_default() as f64)
            }
            BME280_TEMPERATURE => {
                let v = data_row.value.parse::<f64>();
                d.bmp280_temperature = v.clone().ok();
                // dp.field(TEMPERATURE, v.unwrap_or_default() as f64)
            }
            HUMIDITY => {
                let v = data_row.value.parse::<f64>();
                d.humidity = v.clone().ok();
                // dp.field(HUMIDITY, v.unwrap_or_default() as f64)
            }
            BME280_HUMIDITY => {
                let v = data_row.value.parse::<f64>();
                d.bmp280_humidity = v.clone().ok();
                // dp.field(HUMIDITY, v.unwrap_or_default() as f64)
            }
            BMP_PRESSURE => {
                let v = data_row.value.parse::<f64>();
                d.bmp_pressure = v.clone().ok();
                // dp.field(PRESSURE, v.unwrap_or_default() as f64)
            }
            BME280_PRESSURE => {
                let v = data_row.value.parse::<f64>();
                d.bmp280_pressure = v.clone().ok();
                // dp.field(PRESSURE, v.unwrap_or_default() as f64)
            }
            SIGNAL => {
                let v = data_row.value.parse::<i64>();
                d.signal = v.clone().ok();
                // dp.field(SIGNAL, v.unwrap_or_default() as i64)
            }
            _ => {}
        };

        let field_name = measure_name_to_field
            .get(&data_row.value_type)
            .unwrap_or_else(|| &data_row.value_type);

        let sensor_type = match measure_name_to_sensor_type.get(&data_row.value_type) {
            Some(s) => s,
            None => {
                tracing::debug!(
                    "Missing sensor type for chip id {} with value type {}, skipping value",
                    chip_id,
                    &data_row.value_type,
                );
                continue;
            }
        };

        let sensor_id = match get_sensor_id(&sensor_cache, chip_id, &sensor_type) {
            Ok(id) => id,
            Err(e) => {
                tracing::error!(
                    "Error trying to get the sensor id for chip id {} and sensor type {}: {}",
                    chip_id,
                    &sensor_type,
                    e,
                );
                continue;
            }
        };

        let v = data_row.value.parse::<f64>();

        if use_influxdb_3 {
            let mut wq = influxdb::Timestamp::Seconds(chrono::Utc::now().timestamp() as u128)
                .into_query(&influxdb3_settings.table);

            wq = wq
                .add_tag(CHIP_ID, chip_id)
                .add_tag(CITY, d.city.clone())
                .add_tag(LAT, d.lat)
                .add_tag(LON, d.lon)
                .add_tag(SENSOR_ID, sensor_id)
                .add_tag(SENSOR_TYPE, sensor_type.to_owned())
                .add_tag(INFO, d.info.clone());

            wq = wq.add_field(field_name.as_str(), v.unwrap_or_default() as f64);
            write_queries.push(wq);
        } else {
            let mut dp = influxdb2::models::DataPoint::builder(&influxdb_settings.measurement);

            dp = dp
                .tag(CHIP_ID, chip_id)
                .tag(CITY, d.city.clone())
                .tag(LAT, d.lat.to_string())
                .tag(LON, d.lon.to_string())
                .tag(SENSOR_ID, sensor_id)
                .tag(SENSOR_TYPE, sensor_type)
                .tag(INFO, d.info.clone());

            dp = dp.field(field_name.as_str(), v.unwrap_or_default() as f64);
            points.push(dp.build()?);
        }
    }

    if let Err(e) = write_csv(file_path, &d) {
        tracing::error!(
            "Error trying to write csv file at {}: {}",
            file_path.as_os_str().to_string_lossy(),
            e
        );
    }

    let req_builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .danger_accept_invalid_certs(true);

    if use_influxdb_3 {
        let mut client =
            influxdb::Client::new(&influxdb3_settings.url, &influxdb3_settings.database);
        if influxdb3_settings.token.len() > 0 {
            client = client.with_token(&influxdb3_settings.token);
        }

        if let Err(e) = client.query(&write_queries).await {
            tracing::error!(
                "Error trying to write to InfluxDB at {}: {}",
                &influxdb3_settings.url,
                e
            );
        }
    } else {
        let builder = influxdb2::ClientBuilder::with_builder(
            req_builder,
            &influxdb_settings.url,
            &influxdb_settings.org,
            &influxdb_settings.token,
        );
        let client = builder.build()?;

        if let Err(e) = client
            .write(&influxdb_settings.bucket, futures::stream::iter(points))
            .await
        {
            tracing::error!(
                "Error trying to write to InfluxDB at {}: {}",
                &influxdb_settings.url,
                e
            );
        }
    }

    Ok(())
}

pub fn write_csv(
    file_path: &std::path::PathBuf,
    d: &crate::sensor_data::DataRecord<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(file_path)?;

    let needs_headers = file.seek(std::io::SeekFrom::End(0))? == 0;

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(needs_headers)
        .from_writer(file);
    wtr.serialize(d)?;

    wtr.flush()?;
    Ok(())
}
