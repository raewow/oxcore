mod components;

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
    components::Route, components::Router, components::Routes, hooks::use_params_map, ParamSegment,
    StaticSegment,
};

use crate::components::ui::{Button, Card, Input, Label};

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
            </Routes>
        </Router>
    }
}

#[component]
fn Home() -> impl IntoView {
    let account = Resource::new(|| (), |_| portal::get_portal_overview());

    view! {
        <main class="min-h-screen bg-background text-foreground">
            <section class="mx-auto flex min-h-screen max-w-5xl flex-col justify-center px-6 py-20">
                <p class="mb-4 text-xs font-semibold uppercase tracking-[0.3em] text-primary">
                    "oxcore"
                </p>
                <h1 class="max-w-3xl font-sans text-5xl font-semibold tracking-tight sm:text-6xl">
                    "The player portal is taking shape."
                </h1>
                <p class="mt-6 max-w-2xl text-lg leading-8 text-muted-foreground">
                    "Account registration, character management, and operations tooling will be available here."
                </p>
                <Suspense fallback=move || view! {
                    <HomeActions authenticated=false />
                }>
                    {move || view! { <HomeActions authenticated=account.get().is_some_and(|result| result.is_ok()) /> }}
                </Suspense>
            </section>
        </main>
    }
}

#[component]
fn HomeActions(authenticated: bool) -> impl IntoView {
    view! {
        <div class="mt-10 flex flex-wrap gap-3 text-sm">
            {authenticated.then(|| view! {
                <a class="inline-flex h-8 items-center justify-center rounded-none bg-primary px-3 text-xs font-medium text-primary-foreground hover:bg-primary/80" href="/account">
                    "My account"
                </a>
            })}
            {(!authenticated).then(|| view! {
                <a class="inline-flex h-8 items-center justify-center rounded-none bg-primary px-3 text-xs font-medium text-primary-foreground hover:bg-primary/80" href="/register">
                    "Create an account"
                </a>
            })}
            {(!authenticated).then(|| view! {
                <a class="inline-flex h-8 items-center justify-center rounded-none border border-input bg-input px-3 text-xs font-medium text-foreground hover:bg-card" href="/login">
                    "Sign in"
                </a>
            })}
            <a class="inline-flex h-8 items-center justify-center rounded-none border border-input bg-input px-3 text-xs font-medium text-foreground hover:bg-card" href="/status">
                "Realm status"
            </a>
        </div>
    }
}

#[component]
fn Account() -> impl IntoView {
    let overview = Resource::new(|| (), |_| portal::get_portal_overview());

    view! {
        <AuthShell title="Your account" subtitle="Account controls and character information will appear here.">
            <Suspense fallback=move || view! {
                <div class="mt-8 border-y border-border py-5 text-xs text-muted-foreground">"Loading account..."</div>
            }>
                {move || overview.get().map(render_portal_overview)}
            </Suspense>
            <form class="mt-6" action="/auth/logout" method="post">
                <Button button_type="submit">"Sign out"</Button>
            </form>
            <a class="mt-5 inline-block text-xs text-primary hover:underline" href="/security">
                "Change password"
            </a>
            <div class="mt-4 flex gap-4 text-xs">
                <a class="text-primary hover:underline" href="/activity">"Account activity"</a>
                <a class="text-primary hover:underline" href="/support">"Support tickets"</a>
            </div>
        </AuthShell>
    }
}

#[component]
fn Activity() -> impl IntoView {
    let activity = Resource::new(|| (), |_| portal::get_account_activity());
    view! {
        <AuthShell title="Account activity" subtitle="Recent security and portal activity for this account.">
            <Suspense fallback=move || view! { <p class="mt-8 text-xs text-muted-foreground">"Loading activity..."</p> }>
                {move || activity.get().map(render_activity)}
            </Suspense>
            <a class="mt-6 inline-block text-xs text-primary hover:underline" href="/account">"Return to account"</a>
        </AuthShell>
    }
}

fn render_activity(result: Result<Vec<portal::ActivityEvent>, ServerFnError>) -> AnyView {
    match result {
        Ok(events) if events.is_empty() => view! {
            <p class="mt-8 text-xs text-muted-foreground">"No recorded portal activity yet."</p>
        }.into_any(),
        Ok(events) => view! {
            <ul class="mt-8 divide-y divide-border">
                {events.into_iter().map(|event| view! {
                    <li class="py-3 text-xs">
                        <p class="font-medium text-foreground">{event.action}</p>
                        <p class="mt-1 text-muted-foreground">{event.target_type} " - " {event.occurred_at}</p>
                    </li>
                }).collect_view()}
            </ul>
        }.into_any(),
        Err(_) => view! { <p class="mt-8 text-xs text-muted-foreground">"Activity is temporarily unavailable."</p> }.into_any(),
    }
}

#[component]
fn Support() -> impl IntoView {
    let tickets = Resource::new(|| (), |_| portal::get_support_tickets());
    view! {
        <AuthShell title="Support" subtitle="Open a portal support request and track its progress here.">
            <form class="mt-8 space-y-5" action="/support/create" method="post">
                <AuthField id="ticket-subject" name="subject" label="Subject" input_type="text" autocomplete="off" />
                <label class="block text-xs font-medium text-foreground" for="ticket-message">
                    "Message"
                    <textarea id="ticket-message" name="message" required maxlength="8000" class="mt-2 min-h-28 w-full border border-input bg-input px-3 py-2 text-sm text-foreground outline-none focus:border-primary" />
                </label>
                <Button button_type="submit" class="w-full">"Submit request"</Button>
            </form>
            <section class="mt-10 border-t border-border pt-6">
                <p class="font-medium text-foreground">"Your requests"</p>
                <Suspense fallback=move || view! { <p class="mt-4 text-xs text-muted-foreground">"Loading requests..."</p> }>
                    {move || tickets.get().map(render_support_tickets)}
                </Suspense>
            </section>
            <a class="mt-6 inline-block text-xs text-primary hover:underline" href="/account">"Return to account"</a>
        </AuthShell>
    }
}

fn render_support_tickets(result: Result<Vec<portal::SupportTicket>, ServerFnError>) -> AnyView {
    match result {
        Ok(tickets) if tickets.is_empty() => view! { <p class="mt-4 text-xs text-muted-foreground">"No support requests yet."</p> }.into_any(),
        Ok(tickets) => view! {
            <ul class="mt-4 divide-y divide-border">
                {tickets.into_iter().map(|ticket| view! {
                    <li class="py-3 text-xs">
                        <p class="font-medium text-foreground">{ticket.subject}</p>
                        <p class="mt-1 text-muted-foreground">"#" {ticket.id} " - " {ticket.status} " - updated " {ticket.updated_at}</p>
                    </li>
                }).collect_view()}
            </ul>
        }.into_any(),
        Err(_) => view! { <p class="mt-4 text-xs text-muted-foreground">"Support requests are temporarily unavailable."</p> }.into_any(),
    }
}

#[component]
fn Status() -> impl IntoView {
    let realms = Resource::new(|| (), |_| portal::get_realm_status());
    view! {
        <AuthShell title="Realm status" subtitle="Current configured realm availability.">
            <Suspense fallback=move || view! { <p class="mt-8 text-xs text-muted-foreground">"Loading realm status..."</p> }>
                {move || realms.get().map(render_realm_status)}
            </Suspense>
            <a class="mt-6 inline-block text-xs text-primary hover:underline" href="/">"Return home"</a>
        </AuthShell>
    }
}

fn render_realm_status(result: Result<Vec<portal::RealmStatus>, ServerFnError>) -> AnyView {
    match result {
        Ok(realms) if realms.is_empty() => view! { <p class="mt-8 text-xs text-muted-foreground">"No realms are configured."</p> }.into_any(),
        Ok(realms) => view! {
            <ul class="mt-8 divide-y divide-border">
                {realms.into_iter().map(|realm| {
                    let state = if realm.online { "Online" } else { "Offline" };
                    view! {
                        <li class="flex items-center justify-between py-3 text-xs">
                            <div><p class="font-medium text-foreground">{realm.name}</p><p class="mt-1 text-muted-foreground">"Population: " {realm.population}</p></div>
                            <span class=if realm.online { "text-primary" } else { "text-muted-foreground" }>{state}</span>
                        </li>
                    }
                }).collect_view()}
            </ul>
        }.into_any(),
        Err(_) => view! { <p class="mt-8 text-xs text-muted-foreground">"Realm status is temporarily unavailable."</p> }.into_any(),
    }
}

#[component]
fn Security() -> impl IntoView {
    let sessions = Resource::new(|| (), |_| portal::get_active_sessions());

    view! {
        <AuthShell title="Account security" subtitle="Changing your password signs out every portal session.">
            <form class="mt-8 space-y-5" action="/auth/change-password" method="post">
                <AuthField id="current-password" name="current_password" label="Current password" input_type="password" autocomplete="current-password" />
                <AuthField id="new-password" name="new_password" label="New password" input_type="password" autocomplete="new-password" />
                <AuthField id="confirm-password" name="confirm_password" label="Confirm new password" input_type="password" autocomplete="new-password" />
                <Button button_type="submit" class="w-full">"Update password"</Button>
            </form>
            <section class="mt-10 border-t border-border pt-6">
                <p class="font-medium text-foreground">"Active sessions"</p>
                <p class="mt-2 text-xs leading-6 text-muted-foreground">"Revoking the current session signs you out immediately."</p>
                <Suspense fallback=move || view! {
                    <p class="mt-4 text-xs text-muted-foreground">"Loading sessions..."</p>
                }>
                    {move || sessions.get().map(render_active_sessions)}
                </Suspense>
            </section>
            <a class="mt-6 inline-block text-xs text-primary hover:underline" href="/account">"Return to account"</a>
        </AuthShell>
    }
}

fn render_active_sessions(result: Result<Vec<portal::SessionSummary>, ServerFnError>) -> AnyView {
    match result {
        Ok(sessions) if sessions.is_empty() => {
            view! { <p class="mt-4 text-xs text-muted-foreground">"No active sessions."</p> }.into_any()
        }
        Ok(sessions) => view! {
            <ul class="mt-4 divide-y divide-border">
                {sessions.into_iter().map(|session| {
                    let label = if session.current { "Current browser" } else { "Other browser" };
                    view! {
                        <li class="flex items-center justify-between gap-4 py-3 text-xs">
                            <div>
                                <p class="font-medium text-foreground">{label}</p>
                                <p class="mt-1 text-muted-foreground">"Last active: " {session.last_seen_at}</p>
                            </div>
                            <form action="/auth/revoke-session" method="post">
                                <input type="hidden" name="session_id" value=session.id />
                                <button class="text-primary hover:underline" type="submit">"Revoke"</button>
                            </form>
                        </li>
                    }
                }).collect_view()}
            </ul>
        }.into_any(),
        Err(_) => view! {
            <p class="mt-4 text-xs text-muted-foreground">"Sessions are temporarily unavailable."</p>
        }.into_any(),
    }
}

fn render_portal_overview(result: Result<portal::PortalOverview, ServerFnError>) -> AnyView {
    match result {
        Ok(overview) => {
            let email = overview
                .email
                .unwrap_or_else(|| "No email address".to_string());
            let email_status = if overview.email_verified {
                "Verified"
            } else {
                "Verification pending"
            };
            let characters = overview.characters.into_iter().map(|character| {
                view! {
                    <li class="flex items-center justify-between border-t border-border py-3 first:border-t-0">
                        <div>
                            <a class="font-medium text-foreground hover:text-primary" href=format!("/characters/{}", character.guid)>{character.name}</a>
                            <p class="mt-1 text-muted-foreground">
                                "Level " {character.level} " " {class_name(character.class)} " - " {race_name(character.race)}
                            </p>
                        </div>
                        <span class=if character.online == 0 { "text-muted-foreground" } else { "text-primary" }>
                            {if character.online == 0 { "Offline" } else { "Online" }}
                        </span>
                    </li>
                }
            });
            view! {
                <div class="mt-8 space-y-6 text-xs">
                    <section class="border-y border-border py-5">
                        <p class="text-muted-foreground">"Signed in as"</p>
                        <p class="mt-1 font-sans text-lg font-semibold text-foreground">{overview.username}</p>
                        <p class="mt-3 text-muted-foreground">{email}</p>
                        <p class="mt-1 text-primary">{email_status}</p>
                    </section>
                    <section>
                        <p class="font-medium text-foreground">"Characters"</p>
                        <ul class="mt-3">{characters.collect_view()}</ul>
                    </section>
                </div>
            }
            .into_any()
        }
        Err(_) => view! {
            <div class="mt-8 border-y border-border py-5 text-xs text-muted-foreground">
                "Account details are temporarily unavailable."
            </div>
        }
        .into_any(),
    }
}

#[component]
fn CharacterDetail() -> impl IntoView {
    let params = use_params_map();
    let character = Resource::new(
        move || {
            params
                .read()
                .get("guid")
                .and_then(|value| value.parse::<u32>().ok())
        },
        |guid| async move {
            match guid {
                Some(guid) => portal::get_character_detail(guid).await,
                None => Ok(None),
            }
        },
    );
    view! {
        <AuthShell title="Character" subtitle="Character details are visible only to the owning account.">
            <Suspense fallback=move || view! { <p class="mt-8 text-xs text-muted-foreground">"Loading character..."</p> }>
                {move || character.get().map(render_character_detail)}
            </Suspense>
            <a class="mt-6 inline-block text-xs text-primary hover:underline" href="/account">"Return to account"</a>
        </AuthShell>
    }
}

fn render_character_detail(
    result: Result<Option<portal::CharacterDetail>, ServerFnError>,
) -> AnyView {
    match result {
        Ok(Some(character)) => view! {
            <dl class="mt-8 divide-y divide-border text-xs">
                <div class="py-3"><dt class="text-muted-foreground">"Name"</dt><dd class="mt-1 font-medium text-foreground">{character.name}</dd></div>
                <div class="py-3"><dt class="text-muted-foreground">"Class"</dt><dd class="mt-1 text-foreground">"Level " {character.level} " " {class_name(character.class)}</dd></div>
                <div class="py-3"><dt class="text-muted-foreground">"Race"</dt><dd class="mt-1 text-foreground">{race_name(character.race)}</dd></div>
                <div class="py-3"><dt class="text-muted-foreground">"Status"</dt><dd class="mt-1 text-foreground">{if character.online == 0 { "Offline" } else { "Online" }}</dd></div>
                <div class="py-3"><dt class="text-muted-foreground">"Played"</dt><dd class="mt-1 text-foreground">{format_played_time(character.played_time_total)}</dd></div>
                <div class="py-3"><dt class="text-muted-foreground">"Copper"</dt><dd class="mt-1 text-foreground">{character.money}</dd></div>
            </dl>
        }.into_any(),
        Ok(None) => view! { <p class="mt-8 text-xs text-muted-foreground">"Character not found."</p> }.into_any(),
        Err(_) => view! { <p class="mt-8 text-xs text-muted-foreground">"Character details are temporarily unavailable."</p> }.into_any(),
    }
}

fn format_played_time(total_seconds: u32) -> String {
    let hours = total_seconds / 3_600;
    let days = hours / 24;
    format!("{days}d {}h", hours % 24)
}

fn class_name(class: u8) -> &'static str {
    match class {
        1 => "Warrior",
        2 => "Paladin",
        3 => "Hunter",
        4 => "Rogue",
        5 => "Priest",
        7 => "Shaman",
        8 => "Mage",
        9 => "Warlock",
        11 => "Druid",
        _ => "Unknown class",
    }
}

fn race_name(race: u8) -> &'static str {
    match race {
        1 => "Human",
        2 => "Orc",
        3 => "Dwarf",
        4 => "Night Elf",
        5 => "Undead",
        6 => "Tauren",
        7 => "Gnome",
        8 => "Troll",
        _ => "Unknown race",
    }
}

#[component]
fn Login() -> impl IntoView {
    view! {
        <AuthShell title="Sign in" subtitle="Use the same account credentials you use in game.">
            <form class="mt-8 space-y-5" action="/auth/login" method="post">
                <AuthField id="login-username" name="username" label="Account name" input_type="text" autocomplete="username" />
                <AuthField id="login-password" name="password" label="Password" input_type="password" autocomplete="current-password" />
                <Button button_type="submit" class="w-full">"Sign in"</Button>
            </form>
            <div class="mt-6 flex justify-between text-sm">
                <a class="text-sky-400 hover:text-sky-300" href="/recover">"Forgot password?"</a>
                <a class="text-slate-400 hover:text-slate-200" href="/register">"Create account"</a>
            </div>
        </AuthShell>
    }
}

#[component]
fn Register() -> impl IntoView {
    view! {
        <AuthShell title="Create your account" subtitle="One password works for both supported game clients and this portal.">
            <form class="mt-8 space-y-5" action="/auth/register" method="post">
                <AuthField id="register-username" name="username" label="Account name" input_type="text" autocomplete="username" />
                <AuthField id="register-email" name="email" label="Email address" input_type="email" autocomplete="email" />
                <AuthField id="register-password" name="password" label="Password" input_type="password" autocomplete="new-password" />
                <Button button_type="submit" class="w-full">"Create account"</Button>
            </form>
            <p class="mt-6 text-sm leading-6 text-slate-400">
                "Email verification will be required once outbound email delivery is configured."
            </p>
            <p class="mt-4 text-sm text-slate-400">
                "Already have an account? "
                <a class="text-sky-400 hover:text-sky-300" href="/login">"Sign in"</a>
            </p>
        </AuthShell>
    }
}

#[component]
fn RecoverAccount() -> impl IntoView {
    view! {
        <AuthShell title="Account recovery" subtitle="Password recovery is not available yet.">
            <p class="mt-8 text-sm leading-6 text-slate-300">
                "Recovery links require outbound email delivery, which has not been configured for this server. Contact a server administrator for account help."
            </p>
            <a class="mt-8 inline-block text-sm text-sky-400 hover:text-sky-300" href="/login">"Return to sign in"</a>
        </AuthShell>
    }
}

#[component]
fn AuthShell(title: &'static str, subtitle: &'static str, children: Children) -> impl IntoView {
    view! {
        <main class="grid min-h-screen place-items-center bg-background px-6 py-12 text-foreground">
            <Card class="w-full max-w-md px-4 py-8 sm:px-6">
                <a class="text-xs font-semibold uppercase tracking-[0.3em] text-primary" href="/">"oxcore"</a>
                <h1 class="mt-6 font-sans text-3xl font-semibold tracking-tight">{title}</h1>
                <p class="mt-3 text-xs leading-6 text-muted-foreground">{subtitle}</p>
                {children()}
            </Card>
        </main>
    }
}

#[component]
fn AuthField(
    id: &'static str,
    name: &'static str,
    label: &'static str,
    input_type: &'static str,
    autocomplete: &'static str,
) -> impl IntoView {
    view! {
        <Label for_id=id>{label}</Label>
        <Input id=id name=name input_type=input_type autocomplete=autocomplete />
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <main class="grid min-h-screen place-items-center bg-slate-950 px-6 text-slate-100">
            <section class="max-w-md text-center">
                <p class="text-sm font-semibold uppercase tracking-[0.3em] text-sky-400">"404"</p>
                <h1 class="mt-4 text-3xl font-semibold">"Page not found"</h1>
                <a class="mt-6 inline-block text-sky-400 hover:text-sky-300" href="/">"Return home"</a>
            </section>
        </main>
    }
}
