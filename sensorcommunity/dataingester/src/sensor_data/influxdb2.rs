use async_trait::async_trait;

use super::DataWriter;
use super::{CHIP_ID, CITY, INFO, LAT, LON, SENSOR_ID, SENSOR_TYPE};

pub struct InfluxDB2DataWriter {
    pub settings: crate::config::InfluxDB,
}

impl InfluxDB2DataWriter {
    pub fn new(settings: crate::config::InfluxDB) -> Self {
        InfluxDB2DataWriter { settings }
    }
}

#[async_trait]
impl DataWriter for InfluxDB2DataWriter {
    async fn write(&self, recs: &[super::Record]) -> anyhow::Result<()> {
        let mut points = vec![];
        let req_builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .danger_accept_invalid_certs(true);
        let builder = influxdb2::ClientBuilder::with_builder(
            req_builder,
            &self.settings.url,
            &self.settings.org,
            &self.settings.token,
        );
        let client = builder.build()?;

        for rec in recs {
            for d in &rec.values {
                let mut dp = influxdb2::models::DataPoint::builder(&self.settings.measurement);

                dp = dp
                    .timestamp(rec.timestamp as i64)
                    .tag(CHIP_ID, rec.chip_id.as_str())
                    .tag(CITY, rec.city.as_str())
                    .tag(LAT, rec.lat.to_string())
                    .tag(LON, rec.lon.to_string())
                    .tag(INFO, rec.info.as_str())
                    .tag(SENSOR_ID, d.sensor_id.as_str())
                    .tag(SENSOR_TYPE, d.sensor_type.as_str());

                dp = dp.field(d.field.as_str(), d.value);
                points.push(dp.build()?);
            }
        }
        client
            .write(&self.settings.bucket, futures::stream::iter(points))
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Error trying to write to InfluxDB at {}: {}",
                    self.settings.url,
                    e
                )
            })?;

        Ok(())
    }
}
