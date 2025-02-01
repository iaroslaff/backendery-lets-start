mod api;
mod configs;

use std::{borrow::Cow, sync::Arc};

use anyhow::Context;
use axum::{
    http::{header, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use config::{Config, File};
use sentry::ClientInitGuard;

use shuttle_axum::ShuttleAxum;
use shuttle_runtime::{
    main as shuttle_main, SecretStore as ShuttleSecretStore, Secrets as ShuttleSecrets,
};
use tower_http::cors::CorsLayer;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::prelude::*;
use validator::Validate;

use crate::api::handlers::{alive_handler, send_message_handler};
use crate::configs::AppConfigs;

#[derive(Clone, Debug)]
pub struct AppState {
    configs: AppConfigs,
}

impl AppState {
    pub fn configs(&self) -> &AppConfigs {
        &self.configs
    }
}

fn sentry_init(configs: &AppConfigs) -> ClientInitGuard {
    let dsn = configs.sentry_dsn.as_str();
    let environment = Some(Cow::Owned(configs.sentry_environment.clone()));

    sentry::init((
        dsn,
        sentry::ClientOptions {
            environment,
            release: sentry::release_name!(),
            traces_sample_rate: 1.0,
            ..Default::default()
        },
    ))
}

fn tracing_init(configs: &AppConfigs) {
    let level_filter =
        configs.tracing_log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::INFO);

    let filter_layer =
        EnvFilter::builder().with_default_directive(level_filter.into()).from_env_lossy();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .flatten_event(true)
        .with_ansi(false)
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::SystemTime)
        .with_span_list(true);

    tracing_subscriber::registry().with(filter_layer).with(fmt_layer).init();
}

#[shuttle_main]
async fn axum(#[ShuttleSecrets] secrets: ShuttleSecretStore) -> ShuttleAxum {
    let secrets_source = Config::try_from(&secrets).context("couldn't get the secrets")?;
    let configs = Config::builder()
        .add_source(File::with_name("configs/default").required(true))
        .add_source(secrets_source)
        .build()
        .context("couldn't get the application config")?
        .try_deserialize::<AppConfigs>()
        .context("failed to deserialise the application config")?;

    configs.validate().context("failed to validate the application config")?;

    let _guard = sentry_init(&configs);
    tracing_init(&configs);

    let app = Router::new()
        .route("/api/v1/alive", get(alive_handler))
        .route("/api/v1/send-message", post(send_message_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(
                    configs
                        .allow_cors_origins
                        .iter()
                        .map(|header| header.parse::<HeaderValue>().unwrap())
                        .collect::<Vec<_>>(),
                )
                .allow_headers([header::ACCEPT, header::CONTENT_TYPE])
                .allow_methods([Method::HEAD, Method::GET, Method::POST]),
        )
        .with_state(Arc::new(AppState { configs }));

    Ok(app.into())
}
