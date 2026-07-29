use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalOverview {
    pub username: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub characters: Vec<PortalCharacter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct PortalCharacter {
    pub guid: u32,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub online: u8,
}

#[server]
pub async fn get_portal_overview() -> Result<PortalOverview, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use axum_extra::extract::cookie::CookieJar;
        use leptos_axum::extract;

        let state = expect_context::<crate::state::AppState>();
        let jar: CookieJar = extract().await?;
        let Some(cookie) = jar.get("oxcore_session") else {
            return Err(ServerFnError::ServerError("Not authenticated".to_string()));
        };
        let Some(session) = crate::auth::session_from_token(&state.web, cookie.value()).await
        else {
            return Err(ServerFnError::ServerError("Not authenticated".to_string()));
        };

        return load_overview(&state, session.account_id)
            .await
            .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[cfg(feature = "ssr")]
pub async fn overview(
    axum::Extension(state): axum::Extension<crate::state::AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let Some(cookie) = jar.get("oxcore_session") else {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(session) = crate::auth::session_from_token(&state.web, cookie.value()).await else {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    };

    match load_overview(&state, session.account_id).await {
        Ok(overview) => axum::Json(overview).into_response(),
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, "portal overview query failed");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(feature = "ssr")]
async fn load_overview(
    state: &crate::state::AppState,
    account_id: u32,
) -> anyhow::Result<PortalOverview> {
    use anyhow::Context;

    let (username, email, email_verified) = sqlx::query_as::<_, (String, Option<String>, bool)>(
        "SELECT `username`, `email`, `email_verif` FROM `account` WHERE `id` = ?",
    )
    .bind(account_id)
    .fetch_optional(&*state.auth)
    .await
    .context("failed to load portal account")?
    .context("portal account no longer exists")?;

    let characters = sqlx::query_as::<_, PortalCharacter>(
        "SELECT `guid`, `name`, `race`, `class`, `level`, `online` \
         FROM `characters` WHERE `account` = ? ORDER BY `guid`",
    )
    .bind(account_id)
    .fetch_all(&*state.characters)
    .await
    .context("failed to load portal characters")?;

    Ok(PortalOverview {
        username,
        email,
        email_verified,
        characters,
    })
}
