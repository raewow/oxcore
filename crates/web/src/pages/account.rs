use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::ui::{AuthField, AuthShell, Button, Card};
use crate::portal;

#[component]
pub fn Account() -> impl IntoView {
    let overview = Resource::new(|| (), |_| portal::get_portal_overview());
    let is_admin = Resource::new(|| (), |_| portal::has_admin_access());

    view! {
        <main class="min-h-screen bg-background px-6 py-12 text-foreground">
            <div class="mx-auto w-full max-w-3xl">
                <a class="text-xs font-semibold uppercase tracking-[0.3em] text-primary" href="/">"oxcore"</a>
                <h1 class="mt-6 font-sans text-3xl font-semibold tracking-tight">"Your account"</h1>
                <p class="mt-3 text-xs leading-6 text-muted-foreground">"Account controls and character information."</p>
                <div class="mt-8 grid gap-4 sm:grid-cols-2">
                    <Suspense fallback=move || view! {
                        <Card class="px-4 py-5 sm:col-span-2"><p class="text-xs text-muted-foreground">"Loading account..."</p></Card>
                    }>
                        {move || overview.get().map(render_portal_overview)}
                    </Suspense>
                    <Card class="px-4 py-5">
                        <p class="font-medium text-foreground">"Security"</p>
                        <p class="text-xs text-muted-foreground">"Change your password or sign out of the portal."</p>
                        <div class="flex flex-col items-start gap-3">
                            <a class="text-primary hover:underline" href="/security">"Change password"</a>
                            <form action="/auth/logout" method="post">
                                <Button button_type="submit">"Sign out"</Button>
                            </form>
                        </div>
                    </Card>
                    <Card class="px-4 py-5">
                        <p class="font-medium text-foreground">"More"</p>
                        <p class="text-xs text-muted-foreground">"Review recent activity, open a support request, or manage the server."</p>
                        <div class="flex flex-col items-start gap-3">
                            <a class="text-primary hover:underline" href="/activity">"Account activity"</a>
                            <a class="text-primary hover:underline" href="/support">"Support tickets"</a>
                            <Suspense fallback=move || view! { <></> }>
                                {move || is_admin.get().map(|result| match result {
                                    Ok(true) => view! {
                                        <a class="text-primary hover:underline" href="/admin">"Admin panel"</a>
                                    }.into_any(),
                                    _ => view! { <></> }.into_any(),
                                })}
                            </Suspense>
                        </div>
                    </Card>
                </div>
            </div>
        </main>
    }
}

#[component]
pub fn Activity() -> impl IntoView {
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
pub fn Support() -> impl IntoView {
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
pub fn Status() -> impl IntoView {
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
pub fn Security() -> impl IntoView {
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
            let character_count = overview.characters.len();
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
                <>
                    <Card class="px-4 py-5">
                        <p class="text-muted-foreground">"Signed in as"</p>
                        <p class="font-sans text-lg font-semibold text-foreground">{overview.username}</p>
                        <div class="text-muted-foreground">
                            <p>{email}</p>
                            <p class="text-primary">{email_status}</p>
                        </div>
                    </Card>
                    <Card class="px-4 py-5">
                        <div class="flex items-baseline justify-between gap-4">
                            <p class="font-medium text-foreground">"Characters"</p>
                            <p class="text-muted-foreground">{character_count} " total"</p>
                        </div>
                        {if character_count == 0 {
                            view! { <p class="text-xs text-muted-foreground">"No characters on this account yet."</p> }.into_any()
                        } else {
                            view! { <ul class="divide-y divide-border">{characters.collect_view()}</ul> }.into_any()
                        }}
                    </Card>
                </>
            }
            .into_any()
        }
        Err(_) => view! {
            <Card class="px-4 py-5 sm:col-span-2">
                <p class="text-xs text-muted-foreground">"Account details are temporarily unavailable."</p>
            </Card>
        }
        .into_any(),
    }
}

#[component]
pub fn CharacterDetail() -> impl IntoView {
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
