use anyhow::{Context, Result};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{extract::Request, extract::State, response::Redirect, response::Response};
use axum::{Extension, Router};
use leptos::config::get_configuration;
use leptos_axum::{generate_route_list, LeptosRoutes};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::config::Config;
use crate::state::AppState;
use crate::{shell, App};

pub async fn serve(config: Config) -> Result<()> {
    config.validate()?;
    let state = AppState::connect(&config).await?;

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
        .route("/auth/login", post(crate::auth::login))
        .route("/auth/register", post(crate::auth::register))
        .route("/auth/logout", post(crate::auth::logout))
        .route("/admin", get(crate::auth::admin))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options)
        .layer(Extension(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            redirect_authenticated_auth_pages,
        ))
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

async fn readyz(Extension(state): Extension<AppState>) -> impl IntoResponse {
    if state.ready().await {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn redirect_authenticated_auth_pages(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if request.method() == Method::GET {
        let path = request.uri().path();
        let is_auth_page = matches!(path, "/login" | "/register" | "/recover");
        let is_account_page = path == "/account";

        if is_auth_page || is_account_page {
            let authenticated = match session_cookie(request.headers()) {
                Some(token) => crate::auth::session_from_token(&state.web, token)
                    .await
                    .is_some(),
                None => false,
            };

            if is_auth_page && authenticated {
                return Redirect::to("/account").into_response();
            }
            if is_account_page && !authenticated {
                return Redirect::to("/login").into_response();
            }
        }
    }

    next.run(request).await
}

fn session_cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|pair| pair.strip_prefix("oxcore_session="))
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
