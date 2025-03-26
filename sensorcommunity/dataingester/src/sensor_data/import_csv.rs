use csv::Reader;
use influxdb::InfluxDbWriteable;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use crate::cache::Cache;
use crate::config::Manifest;

use super::{CHIP_ID, CITY, INFO, LAT, LON, SENSOR_ID, SENSOR_TYPE, TIMESTAMP, get_sensor_id};

// Structure to hold our CSV data
pub struct CsvData {
    _filename: String,
    pub record_count: i64,
}

// Function to load an individual CSV file
pub async fn import_csv(
    path: &Path,
    // influxdb3_settings: &crate::config::InfluxDB3,
    config: &Manifest,
    sensor_cache: Cache<crate::SensorInfo>,
) -> Result<CsvData, Box<dyn Error>> {
    /*
        chip_id,sensor_id,sensor_type,lat,lon,timestamp,P1,ratioP1,durP1,P2,ratioP2,durP2,SDS_P1,SDS_P2,temperature,humidity,BMP_temperature,BMP_pressure,BME280_temperature,BMP280_humidity,BMP280_pressure,signal,city,info
    esp8266-15303512,,,45.630739,11.703086,1742650096,,,,,,,18.95,12.75,12.2,53.3,,,,,,-63,Carmignano di Brenta,centro nord
         */

    // Get filename as string
    let filename = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("unknown"));

    let empty_csv_data = CsvData {
        _filename: filename.clone(),
        // headers,
        record_count: 0,
    };

    let mut fields = HashMap::new();

    let mut reader = Reader::from_path(path)?;

    // Get headers
    for (i, h) in reader.headers()?.iter().enumerate() {
        fields.insert(h.to_string(), i);
    }

    let chip_id_idx = match fields.get(CHIP_ID) {
        None => return Ok(empty_csv_data),
        Some(idx) => *idx,
    };
    let city_idx = match fields.get(CITY) {
        None => return Ok(empty_csv_data),
        Some(idx) => *idx,
    };
    let lat_idx = match fields.get(LAT) {
        None => return Ok(empty_csv_data),
        Some(idx) => *idx,
    };
    let lon_idx = match fields.get(LON) {
        None => return Ok(empty_csv_data),
        Some(idx) => *idx,
    };
    let info_idx = match fields.get(INFO) {
        None => return Ok(empty_csv_data),
        Some(idx) => *idx,
    };
    let timestamp_idx = match fields.get(TIMESTAMP) {
        None => return Ok(empty_csv_data),
        Some(idx) => *idx,
    };

    // Read all records

    let mut write_queries = Vec::<influxdb::WriteQuery>::new();

    for result in reader.records() {
        if let Ok(record) = result {
            let timestamp = match record.get(timestamp_idx) {
                None => {
                    tracing::error!("Missing {} value in file: {}", TIMESTAMP, filename,);
                    continue;
                }
                Some(t) => {
                    let ts = t.parse::<u128>();
                    if ts.is_err() {
                        tracing::error!(
                            "Can not convert to u128 value {} for field {} in file: {}",
                            t,
                            TIMESTAMP,
                            filename,
                        );
                        continue;
                    }
                    ts.unwrap()
                }
            };
            let chip_id = match record.get(chip_id_idx) {
                None => {
                    tracing::error!("Missing {} value in file: {}", CHIP_ID, filename,);
                    continue;
                }
                Some(c) => c,
            };
            let city = match record.get(city_idx) {
                None => {
                    tracing::error!("Missing {} value in file: {}", CITY, filename,);
                    continue;
                }
                Some(c) => c,
            };
            let lat = match record.get(lat_idx) {
                None => {
                    tracing::error!("Missing {} value in file: {}", LAT, filename,);
                    continue;
                }
                Some(l) => {
                    let lat = l.parse::<f64>();
                    if lat.is_err() {
                        tracing::error!(
                            "Can not convert to f64 value {} for field {} in file: {}",
                            l,
                            LAT,
                            filename,
                        );
                        continue;
                    }
                    lat.unwrap()
                }
            };
            let lon = match record.get(lon_idx) {
                None => {
                    tracing::error!("Missing {} value in file: {}", LON, filename,);
                    continue;
                }
                Some(l) => {
                    let lon = l.parse::<f64>();
                    if lon.is_err() {
                        tracing::error!(
                            "Can not convert to f64 value {} for field {} in file: {}",
                            l,
                            LON,
                            filename,
                        );
                        continue;
                    }
                    lon.unwrap()
                }
            };
            let info = match record.get(info_idx) {
                None => {
                    tracing::error!("Missing {} value in file: {}", INFO, filename,);
                    continue;
                }
                Some(i) => i,
            };

            for (measure, sensor_type) in &config.measure_name_to_sensor_type {
                if let Some(f) = config.measure_name_to_field.get(measure) {
                    let field_idx = match fields.get(measure) {
                        None => continue,
                        Some(idx) => *idx,
                    };
                    let value = match record.get(field_idx) {
                        None => continue,
                        Some(s) => {
                            let v = s.parse::<f64>();
                            if v.is_err() {
                                tracing::error!(
                                    "Can not convert to f64 value {} for field {} in file: {}",
                                    s,
                                    &measure,
                                    filename,
                                );
                                continue;
                            }
                            v.unwrap()
                        }
                    };

                    let sensor_id = match get_sensor_id(&sensor_cache, chip_id, sensor_type) {
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

                    let mut wq =
                        influxdb::Timestamp::Seconds(timestamp).into_query(&config.influxdb3.table);
                    wq = wq
                        .add_tag(CHIP_ID, chip_id)
                        .add_tag(CITY, city)
                        .add_tag(LAT, lat)
                        .add_tag(LON, lon)
                        .add_tag(SENSOR_ID, sensor_id.to_owned())
                        .add_tag(SENSOR_TYPE, sensor_type.to_owned())
                        .add_tag(INFO, info)
                        .add_field(f, value);

                    write_queries.push(wq);
                }
            }
        }
    }

    let influxdb3_settings = &config.influxdb3;
    let mut client = influxdb::Client::new(&influxdb3_settings.url, &influxdb3_settings.database);
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

    Ok(CsvData {
        _filename: filename,
        record_count: write_queries.len() as i64,
    })
}
