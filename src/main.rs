mod api;
mod configs;
mod errors;
mod middlewares;
mod models;

use std::sync::Arc;

use axum::{
    http::{header, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use dd_tracing_layer::{DatadogOptions, Region};
use shuttle_axum::ShuttleAxum;
use shuttle_runtime::{
    main as shuttle_main, Error as ShuttleError, SecretStore as ShuttleSecretStore,
    Secrets as ShuttleSecrets,
};
use tower_http::{
    cors::{Any, CorsLayer},
    propagate_header::PropagateHeaderLayer,
};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::prelude::*;

use crate::api::handlers::{index_handler, send_message_handler};
use crate::configs::AppConfigs;
use crate::errors::SecretError;
use crate::middlewares::add_request_id;

#[derive(Clone, Debug)]
pub struct AppState {
    configs: AppConfigs,
    secrets: AppSecrets,
}

impl AppState {
    pub fn configs(&self) -> &AppConfigs {
        &self.configs
    }

    pub fn secrets(&self) -> &AppSecrets {
        &self.secrets
    }
}

#[derive(Clone, Debug, Default)]
pub struct AppSecrets {
    // Datadog
    pub dd_api_key: String,
    pub dd_log_level: String,
    pub dd_tags: String,
    // Smtp
    pub smtp_addr: String,
    pub smtp_auth: String,
}

impl AppSecrets {
    fn new(store: &ShuttleSecretStore) -> Result<Self, SecretError> {
        // Datadog
        /* getting the Datadog Key from the secrets */
        let dd_api_key = store
            .get("DATADOG_API_KEY")
            .ok_or(SecretError::MissingSecret("DATADOG_API_KEY"))?;
        /* getting the Datadog Log Level from the secrets */
        let dd_log_level = store
            .get("DATADOG_LOG_LEVEL")
            .ok_or(SecretError::MissingSecret("DATADOG_LOG_LEVEL"))?;
        /* getting the Datadog Tags from the secrets */
        let dd_tags = store
            .get("DATADOG_TAGS")
            .ok_or(SecretError::MissingSecret("DATADOG_TAGS"))?;

        // Smtp
        /* getting the Smtp Addr from the secrets */
        let smtp_addr = store
            .get("SMTP_ADDR")
            .ok_or(SecretError::MissingSecret("SMTP_ADDR"))?;
        /* getting the Smtp Auth from the secrets */
        let smtp_auth = store
            .get("SMTP_AUTH")
            .ok_or(SecretError::MissingSecret("SMTP_AUTH"))?;

        Ok(Self {
            // Datadog
            dd_api_key,
            dd_log_level,
            dd_tags,
            // Smtp
            smtp_addr,
            smtp_auth,
        })
    }
}

fn configure_tracing(secrets: &AppSecrets) {
    let log_layer = dd_tracing_layer::create(
        DatadogOptions::new("backendery-lets-start", &secrets.dd_api_key)
            .with_region(Region::EU)
            .with_tags(&secrets.dd_tags),
    );

    let level_filter = secrets.dd_log_level.parse::<LevelFilter>()
        .unwrap_or(LevelFilter::INFO);

    let filter_layer = EnvFilter::builder()
        .with_default_directive(level_filter.into())
        .from_env_lossy();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .json()
        .flatten_event(true)
        .with_target(true)
        .with_span_list(true);

    /* starting the tracing subscriber */
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(log_layer)
        .init();
}

fn create_cors_layer(configs: &AppConfigs) -> CorsLayer {
    let origins: Vec<HeaderValue> = (configs.allow_cors_origins)
        .iter()
        .filter(|x| !x.is_empty())
        .map(|x| {
            x.parse::<HeaderValue>()
                .unwrap_or_else(|_| HeaderValue::from_static("localhost"))
        })
        .collect();

    let cors_layer = if origins.is_empty() {
        CorsLayer::new().allow_origin(Any)
    } else {
        CorsLayer::new().allow_origin(origins)
    };

    cors_layer
        .allow_headers([
            header::ACCEPT,
            header::CONTENT_TYPE
        ])
        .allow_methods([
            Method::HEAD,
            Method::GET,
            Method::POST
        ])
}

#[shuttle_main]
async fn axum(#[ShuttleSecrets] store: ShuttleSecretStore) -> ShuttleAxum {
    let configs = match AppConfigs::new("configs/Default") {
        Ok(config) => config,
        Err(cause) => return Err(ShuttleError::Custom(cause.into())),
    };
    let secrets = match AppSecrets::new(&store) {
        Ok(secret) => secret,
        Err(cause) => return Err(ShuttleError::Custom(cause.into())),
    };

    configure_tracing(&secrets);

    let cors_layer = create_cors_layer(&configs);
    let propagate_header_layer =
        PropagateHeaderLayer::new(header::HeaderName::from_static("x-request-id"));
    let request_id_layer = axum::middleware::from_fn(add_request_id);

    let state = Arc::new(AppState { configs, secrets });

    let router: Router = Router::new()
        .route("/", get(index_handler))
        .route("/api/v1/send-message", post(send_message_handler))
        .layer(cors_layer)
        .layer(propagate_header_layer)
        .layer(request_id_layer)
        .with_state(state);

    Ok(router.into())
}
