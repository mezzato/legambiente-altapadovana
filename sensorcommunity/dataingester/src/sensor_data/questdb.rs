use super::DataWriter;
use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use questdb::ingress::{Buffer, Sender, TimestampNanos};

use super::{CHIP_ID, CITY, FIELD, INFO, LAT, LON, SENSOR_ID, SENSOR_TYPE, VALUE};
pub struct QuestDBDataWriter {
    pub settings: crate::config::QuestDB,
}

impl QuestDBDataWriter {
    pub fn new(settings: crate::config::QuestDB) -> Self {
        QuestDBDataWriter { settings }
    }
}

#[async_trait]
impl DataWriter for QuestDBDataWriter {
    async fn write(&self, recs: &[super::Record]) -> anyhow::Result<()> {
        let schema = match self.settings.use_https {
            true => "https",
            false => "http",
        };
        let mut sender = Sender::from_conf(format!(
            "{}::addr={};username={};password={};",
            schema, self.settings.addr, self.settings.username, self.settings.password
        ))?;

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
}
