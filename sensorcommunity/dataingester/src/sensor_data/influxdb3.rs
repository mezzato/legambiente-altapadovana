use super::DataWriter;
use async_trait::async_trait;
use influxdb::InfluxDbWriteable;

use super::{CHIP_ID, CITY, INFO, LAT, LON, SENSOR_ID, SENSOR_TYPE};
pub struct InfluxDB3DataWriter {
    pub settings: crate::config::InfluxDB3,
}

impl InfluxDB3DataWriter {
    pub fn new(settings: crate::config::InfluxDB3) -> Self {
        InfluxDB3DataWriter { settings }
    }
}

#[async_trait]
impl DataWriter for InfluxDB3DataWriter {
    async fn write<'b>(&self, rec: &super::Record<'b>) -> anyhow::Result<()> {
        let mut write_queries = Vec::<influxdb::WriteQuery>::new();

        for d in &rec.values {
            let mut wq = influxdb::Timestamp::Seconds(chrono::Utc::now().timestamp() as u128)
                .into_query(&self.settings.table);

            wq = wq
                .add_tag(CHIP_ID, rec.chip_id)
                .add_tag(CITY, rec.city)
                .add_tag(LAT, rec.lat)
                .add_tag(LON, rec.lon)
                .add_tag(INFO, rec.info)
                .add_tag(SENSOR_ID, d.sensor_id.to_owned())
                .add_tag(SENSOR_TYPE, d.sensor_type.to_owned());

            wq = wq.add_field(d.field.as_str(), d.value);
            write_queries.push(wq);
        }

        let mut client = influxdb::Client::new(&self.settings.url, &self.settings.database);
        if self.settings.token.len() > 0 {
            client = client.with_token(&self.settings.token);
        }

        let _ = client.query(&write_queries).await.map_err(|e| {
            anyhow::anyhow!(
                "Error trying to write to InfluxDB at {}: {}",
                self.settings.url,
                e
            )
        })?;

        Ok(())
    }
}
