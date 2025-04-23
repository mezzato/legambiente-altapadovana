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
    async fn write<'b>(&self, rec: &super::Record<'b>) -> anyhow::Result<()> {
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

        for d in &rec.values {
            let mut dp = influxdb2::models::DataPoint::builder(&self.settings.measurement);

            dp = dp
                .tag(CHIP_ID, rec.chip_id)
                .tag(CITY, rec.city)
                .tag(LAT, rec.lat.to_string())
                .tag(LON, rec.lon.to_string())
                .tag(INFO, rec.info)
                .tag(SENSOR_ID, d.sensor_id.to_owned())
                .tag(SENSOR_TYPE, d.sensor_type);

            dp = dp.field(d.field.as_str(), d.value);
            points.push(dp.build()?);
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
