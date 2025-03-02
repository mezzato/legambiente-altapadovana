use crate::{config::{self}, sensor_data, ChipInfo, SensorData};
use axum::{
    Json, RequestPartsExt, extract::{FromRef, FromRequest, Request, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{
    TypedHeader,
    headers::{
        Authorization,
        authorization::Basic,
    },
};

use crate::cache::Cache;
use anyhow::{Result, anyhow};
use serde_json::json;
use std::{collections::HashMap, path::PathBuf};



// Use anyhow, define error and enable '?'
// For a simplified example of using anyhow in axum check /examples/anyhow-error-response
#[derive(Debug)]
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("Application error: {:#}", self.0);

        (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}


#[derive(Clone)]
pub struct ReqState {
    pub chip_info_cache: Cache<ChipInfo>,
    pub sensor_data_dir: PathBuf,
    pub measure_name_to_field: HashMap<String, String>,
    pub influxdb_settings: config::InfluxDB,
    pub logins: HashMap<String, String>,
}

pub async fn handler(
    State(ReqState {
        chip_info_cache,
        sensor_data_dir,
        measure_name_to_field,
        influxdb_settings,
        logins: _,
    }): State<ReqState>,

    SensorData { json, sensor }: SensorData<sensor_data::Payload>,
) -> Result<(), AppError> {
    // tracing::debug!(?sensor, "sensor");
    // tracing::debug!(?json, "json body");
    // println!("sensor: {}, json: {:?}", sensor, json);

    let formatted_day = format!("{}", chrono::Utc::now().format("%Y-%m-%d"));

    let root_folder = sensor_data_dir.join(&formatted_day);
    let file_name = format!("{}_chip_{}.csv", &formatted_day, &sensor);

    if let Err(e) = std::fs::create_dir_all(&root_folder) {
        tracing::error!(
            "Error creating sensor data folder at: {}, {}",
            root_folder.as_os_str().to_string_lossy(),
            e
        );
        return Err(AppError(anyhow!("{}", e)));
    }

    let file_path = root_folder.join(file_name);

    match sensor_data::write(
        &influxdb_settings,
        &file_path,
        &measure_name_to_field,
        chip_info_cache,
        &sensor,
        json,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("Error trying to write data for sensor {}: {}", &sensor, e);
        }
    };

    Ok(())

    /*
    wtr.write_record(&[
        "Time",
        ;durP1;ratioP1;P1;durP2;ratioP2;P2;SDS_P1;SDS_P2;Temp;Humidity;BMP_temperature;BMP_pressure;BME280_temperature;BME280_humidity;BME280_pressure;Samples;Min_cycle;Max_cycle;Signal\n"
    ])?;
    wtr.write_record(&[
        "Davidsons Landing",
        "AK",
        "",
        "65.2419444",
        "-165.2716667",
    ])?;
    wtr.write_record(&["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])?;
    wtr.write_record(&["Oakman", "AL", "", "33.7133333", "-87.3886111"])?;

    wtr.flush()?;
    */
}

// extractor that shows how to consume the request body upfront
// struct BufferRequestBody(Bytes);

const X_SENSOR_HEADER: &str = "x-sensor";

// the state your library needs

impl<S, T> FromRequest<S> for SensorData<T>
where
    S: Send + Sync,
    Json<T>: FromRequest<()>,
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    T: 'static,
    ReqState: FromRef<S>,
{
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // tracing::debug!(request = ?req);

        // Extract the token from the authorization header
        let sensor_header = req.headers().get(X_SENSOR_HEADER);
        let sensor = sensor_header.and_then(|value| value.to_str().ok());
        let sensor = sensor.unwrap_or_default().to_owned();

        let (mut parts, body) = req.into_parts();

        let creds = match parts.extract::<TypedHeader<Authorization<Basic>>>().await {
            Ok(TypedHeader(Authorization(bearer))) => bearer,
            Err(_) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "missing credentials",
                    })),
                ));
            }
        };

        let mystate: ReqState = ReqState::from_ref(state);

        let pwd = mystate
            .logins
            .get(&creds.username().to_lowercase())
            .ok_or((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "wrong credentials",
                })),
            ))?;

        if pwd != creds.password() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            ));
        };

        // tracing::debug!(headers = ?parts.headers);
        // tracing::debug!(body = ?body);

        // We can use other extractors to provide better rejection messages.
        // For example, here we are using `axum::extract::MatchedPath` to
        // provide a better error message.
        //
        // Have to run that first since `Json` extraction consumes the request.
        let path = parts
            .extract::<axum::extract::MatchedPath>()
            .await
            .map(|path| path.as_str().to_owned())
            .ok();

        let req = Request::from_parts(parts, body);

        let json = match axum::Json::<T>::from_request(req, state).await {
            Ok(value) => Ok(value.0),
            // convert the error from `axum::Json` into whatever we want
            Err(rejection) => {
                // println!("--- rejection: {}", rejection.body_text());
                let payload = json!({
                    "message": rejection.body_text(),
                    "origin": "custom_extractor",
                    "path": path,
                });

                Err((rejection.status(), axum::Json(payload)))
            }
        }?;

        let data = SensorData { json, sensor };

        Ok(data)
    }
}
