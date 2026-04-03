use super::DataWriter;
use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use questdb::ingress::{Buffer, Sender, TimestampNanos};

use super::{CHIP_ID, CITY, FIELD, INFO, LAT, LON, SENSOR_ID, SENSOR_TYPE, TIMESTAMP, VALUE};
pub struct QuestDBDataWriter {
    pub settings: crate::config::QuestDB,
}

impl QuestDBDataWriter {
    pub fn new(settings: crate::config::QuestDB) -> Self {
        QuestDBDataWriter { settings }
    }

    fn conn_string(&self) -> String {
        let schema = match self.settings.use_https {
            true => "https",
            false => "http",
        };
        format!(
            "{}::addr={};username={};password={};",
            schema, self.settings.addr, self.settings.username, self.settings.password
        )
    }

    fn rest_base_url(&self) -> String {
        let schema = match self.settings.use_https {
            true => "https",
            false => "http",
        };
        format!("{}://{}", schema, self.settings.addr)
    }
}

#[async_trait]
impl DataWriter for QuestDBDataWriter {
    async fn write(&self, recs: &[super::Record]) -> anyhow::Result<()> {
        let mut sender = Sender::from_conf(self.conn_string())?;

        let mut buffer = Buffer::new();

        let table = self.settings.table.as_str();

        for rec in recs {
            let dt: DateTime<Utc> = DateTime::from_timestamp(rec.timestamp as i64, 0)
                .ok_or(anyhow!("invalid timestamp: {}", rec.timestamp))?;
            for d in &rec.values {
                buffer
                    .table(table)?
                    .symbol(CHIP_ID, rec.chip_id.to_owned())?
                    .symbol(CITY, rec.city.to_owned())?
                    .symbol(LAT, rec.lat.to_string())?
                    .symbol(LON, rec.lon.to_string())?
                    .symbol(INFO, rec.info.to_owned())?
                    .symbol(SENSOR_ID, d.sensor_id.to_owned())?
                    .symbol(SENSOR_TYPE, d.sensor_type.to_owned())?
                    .symbol(FIELD, d.field.as_str())?
                    .column_f64(VALUE, d.value)?
                    .at(TimestampNanos::from_datetime(dt)?)?;
            }
        }

        sender.flush(&mut buffer)?;

        Ok(())
    }

    async fn refresh_sensor_info(&self, recs: &[super::SensorInfoRecord]) -> anyhow::Result<()> {
        let sensor_info_table = &self.settings.sensor_info_table;
        if sensor_info_table.is_empty() {
            return Ok(());
        }

        // Truncate the sensor_info table via QuestDB REST API
        let base_url = self.rest_base_url();
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        // Create the table if it does not exist
        let create_query = format!(
            "CREATE TABLE IF NOT EXISTS '{}' (\
             {CHIP_ID} SYMBOL, \
             {SENSOR_ID} SYMBOL, \
             {SENSOR_TYPE} SYMBOL, \
             {LAT} DOUBLE, \
             {LON} DOUBLE, \
             {CITY} STRING, \
             {INFO} STRING, \
             {TIMESTAMP} TIMESTAMP\
             ) TIMESTAMP({TIMESTAMP}) PARTITION BY DAY WAL",
            sensor_info_table
        );
        let resp = client
            .get(format!("{}/exec", base_url))
            .query(&[("query", &create_query)])
            .send()
            .await
            .map_err(|e| anyhow!("failed to create table '{}': {}", sensor_info_table, e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "failed to create table '{}': {}",
                sensor_info_table,
                body
            ));
        }

        // Truncate
        let truncate_query = format!("TRUNCATE TABLE '{}'", sensor_info_table);
        let resp = client
            .get(format!("{}/exec", base_url))
            .query(&[("query", &truncate_query)])
            .send()
            .await
            .map_err(|e| anyhow!("failed to truncate table '{}': {}", sensor_info_table, e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "failed to truncate table '{}': {}",
                sensor_info_table,
                body
            ));
        }

        if recs.is_empty() {
            return Ok(());
        }

        // Write all sensor info records via ILP
        let mut sender = Sender::from_conf(self.conn_string())?;
        let mut buffer = Buffer::new();

        for rec in recs {
            buffer
                .table(sensor_info_table.as_str())?
                .symbol(CHIP_ID, &rec.chip_id)?
                .symbol(SENSOR_ID, &rec.sensor_id)?
                .symbol(SENSOR_TYPE, &rec.sensor_type)?
                .column_f64(LAT, rec.lat)?
                .column_f64(LON, rec.lon)?
                .column_str(CITY, &rec.city)?
                .column_str(INFO, &rec.info)?
                .at(TimestampNanos::now())?;
        }

        sender.flush(&mut buffer)?;

        tracing::info!(
            "refreshed sensor_info table '{}' with {} records",
            sensor_info_table,
            recs.len()
        );

        Ok(())
    }
}
