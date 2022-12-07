use axum::{extract::FromRef, middleware, routing::get, Router, Server};
use chrono::NaiveDateTime;
use docker_api::Docker;
use include_dir::{include_dir, Dir};
use log::{debug, info};
use std::{fs, sync::Arc};
use tokio::sync::RwLock;

use crate::config::Config;

mod backup;
mod config;
mod docker;
mod error;
mod handlers;
mod valve;

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

#[derive(Clone, FromRef)]
struct AppState {
    docker: Arc<Docker>,
    a2s_client: Arc<a2s::A2SClient>,
    last_restart_time: Option<NaiveDateTime>,
    template: String,
    config: Arc<Config>,
}

type SharedState = Arc<RwLock<AppState>>;

#[derive(Debug, Clone)]
struct SimpleDirEntry {
    name: String,
    creation_time: NaiveDateTime,
    hr_size: String,
}

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).expect("logging initialization");

    info!(
        "Starting {} {}",
        env!("CARGO_PKG_NAME"),
        version_with_commit()
    );

    let config = Arc::new(Config::new().expect("config initialization"));

    let docker = Arc::new(Docker::unix(&config.docker_socket_path));
    let a2s_client = Arc::new(a2s::A2SClient::new().await.expect("creating A2S client"));
    let template = fs::read_to_string(&config.template_path).expect("loading page template");

    let shared_state: SharedState = Arc::new(RwLock::new(AppState {
        docker,
        a2s_client,
        last_restart_time: None,
        template,
        config: config.clone(),
    }));

    let app = Router::new()
        .route("/", get(handlers::root_handler))
        .route("/restart", get(handlers::restart_handler))
        .route("/backups/:name", get(handlers::backups_handler))
        .route(
            "/backups/restore/:name",
            get(handlers::backups_restore_handler),
        )
        .route("/static/*path", get(handlers::static_path))
        .route_layer(middleware::from_fn_with_state(
            shared_state.clone(),
            handlers::auth,
        ))
        .with_state(shared_state.clone());

    info!("Listening on {}...", config.server_address);

    Server::bind(&config.server_address)
        .serve(app.into_make_service())
        .await
        .expect("starting web server");
}

pub(crate) fn version_with_commit() -> String {
    format!(
        "{}-{}",
        env!("CARGO_PKG_VERSION"),
        env!("VERGEN_GIT_SHA_SHORT")
    )
}
