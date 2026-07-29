use anyhow::{Context, Result};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use leptos::config::get_configuration;
use leptos_axum::{generate_route_list, LeptosRoutes};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::config::Config;
use oxcore_web::{shell, App};

pub async fn serve(config: Config) -> Result<()> {
    config.validate()?;

    let address = config.socket_addr();
    // cargo-leptos injects its configuration through environment variables. Supplying this
    // manifest path also makes a plain `cargo run --bin web` load the same metadata.
    let mut leptos_options =
        get_configuration(Some(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")))
            .context("failed to load Leptos configuration")?
            .leptos_options;
    leptos_options.site_addr = address;
    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind web server to {address}"))?;
    info!(target: "oxcore_web", %address, public_base_url = %config.public_base_url, "web server listening");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("web server failed")
}

async fn healthz() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

async fn readyz() -> impl IntoResponse {
    // Database readiness is added with the account/session stores in Phase 2.
    StatusCode::NO_CONTENT
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }

    #[cfg(not(unix))]
    tokio::signal::ctrl_c()
        .await
        .expect("install Ctrl+C handler");
}
