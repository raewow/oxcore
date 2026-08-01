#![recursion_limit = "256"]

mod components;
mod pages;

#[cfg(feature = "ssr")]
pub mod auth;
#[cfg(feature = "ssr")]
pub mod config;
pub mod portal;
#[cfg(feature = "ssr")]
pub mod server;
#[cfg(feature = "ssr")]
pub mod state;

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::Route, components::Router, components::Routes, ParamSegment, StaticSegment,
};

use crate::pages::account::{Account, Activity, CharacterDetail, Security, Status, Support};
use crate::pages::admin::accounts::{AccountManagement, AdminAccountDetail};
use crate::pages::admin::audit::AdminAuditLog;
use crate::pages::admin::characters::{AdminCharacterDetail, AdminCharacters};
use crate::pages::admin::chat::AdminChat;
use crate::pages::admin::live_map::LiveMap;
use crate::pages::admin::overview::Admin;
use crate::pages::admin::permissions::Permissions;
use crate::pages::auth::{Login, RecoverAccount, Register};
use crate::pages::home::Home;
use crate::pages::not_found::NotFound;

#[cfg(feature = "ssr")]
pub async fn run() -> anyhow::Result<()> {
    use anyhow::Context;
    use oxcore_shared::config::{find_config_file, load_toml};

    let config_path = std::env::args()
        .nth(1)
        .map(Into::into)
        .unwrap_or_else(find_config_file);
    let root: config::RootConfig = load_toml(&config_path).with_context(|| {
        format!(
            "failed to load configuration from {}",
            config_path.display()
        )
    })?;
    let config = root.web.context("[web] config section missing")?;

    server::serve(config).await
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}

/// Document shell used by Axum when server-side rendering a route.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="theme-color" content="#1a1a2e"/>
                <meta name="description" content="oxcore World of Warcraft server portal and administration console"/>
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous"/>
                <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Cinzel:wght@400;500;600;700&family=Geist+Mono:wght@400;500&family=Inter:wght@400;500;600;700&display=swap"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Root Leptos application shared by server-side rendering and browser hydration.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="web" href="/pkg/web.css" />
        <Title text="oxcore" />
        <Router>
            <Routes fallback=|| view! { <NotFound /> }>
                <Route path=StaticSegment("") view=Home />
                <Route path=StaticSegment("login") view=Login />
                <Route path=StaticSegment("register") view=Register />
                <Route path=StaticSegment("recover") view=RecoverAccount />
                <Route path=StaticSegment("account") view=Account />
                <Route path=(StaticSegment("characters"), ParamSegment("guid")) view=CharacterDetail />
                <Route path=StaticSegment("security") view=Security />
                <Route path=StaticSegment("activity") view=Activity />
                <Route path=StaticSegment("support") view=Support />
                <Route path=StaticSegment("status") view=Status />
                <Route path=StaticSegment("admin") view=Admin />
                <Route path=(StaticSegment("admin"), StaticSegment("accounts")) view=AccountManagement />
                <Route path=(StaticSegment("admin"), StaticSegment("accounts"), ParamSegment("account_id")) view=AdminAccountDetail />
                <Route path=(StaticSegment("admin"), StaticSegment("characters")) view=AdminCharacters />
                <Route path=(StaticSegment("admin"), StaticSegment("characters"), ParamSegment("guid")) view=AdminCharacterDetail />
                <Route path=(StaticSegment("admin"), StaticSegment("audit-logs")) view=AdminAuditLog />
                <Route path=(StaticSegment("admin"), StaticSegment("chat")) view=AdminChat />
                <Route path=(StaticSegment("admin"), StaticSegment("permissions")) view=Permissions />
                <Route path=(StaticSegment("admin"), StaticSegment("live-map")) view=LiveMap />
            </Routes>
        </Router>
    }
}
