use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::ui::Button;
use crate::pages::admin::shell::{AdminSection, AdminShell};
use crate::portal;

#[component]
pub fn AccountManagement() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let page = RwSignal::new(1_u32);
    let accounts = Resource::new(
        move || (search.get(), page.get()),
        |(search, page)| portal::get_admin_accounts(Some(search), page),
    );
    view! {
        <AdminShell active=AdminSection::Accounts>
            <p class="text-xs font-semibold uppercase tracking-[0.3em] text-primary">"Account management"</p>
            <h1 class="mt-4 font-sans text-3xl font-semibold tracking-tight">"Accounts"</h1>
            <p class="mt-3 text-xs text-muted-foreground">"Search accounts by name, email, or ID. Credentials, session keys, and recovery secrets are never shown here."</p>
            <label class="mt-6 block max-w-md text-xs text-muted-foreground" for="account-search">"Search"
                <input class="mt-2 block w-full border border-input bg-input px-3 py-2 text-foreground" id="account-search" type="search" placeholder="Account name, email, or ID" on:input=move |event| { search.set(event_target_value(&event)); page.set(1); } />
            </label>
            <Suspense fallback=move || view! { <p class="mt-8 text-xs text-muted-foreground">"Loading accounts..."</p> }>
                {move || accounts.get().map(move |result| render_admin_accounts(result, page))}
            </Suspense>
        </AdminShell>
    }
}

fn render_admin_accounts(
    result: Result<portal::AdminAccountPage, ServerFnError>,
    page: RwSignal<u32>,
) -> AnyView {
    match result {
        Ok(result) => {
            let has_next = result.page as u64 * (result.page_size as u64) < result.total;
            let current_page = result.page;
            let accounts = result.accounts;
            view! {
            <div class="mt-8 overflow-x-auto border border-border">
                <table class="min-w-full text-left text-xs">
                    <thead class="border-b border-border bg-card text-muted-foreground"><tr><th class="px-4 py-3">"ID"</th><th class="px-4 py-3">"Account"</th><th class="px-4 py-3">"Email"</th><th class="px-4 py-3">"Role"</th><th class="px-4 py-3">"State"</th></tr></thead>
                    <tbody>{accounts.into_iter().map(|account| {
                        let state = if account.banned != 0 { "Banned" } else if account.locked != 0 { "Locked" } else { "Active" };
                        let email = account.email.unwrap_or_else(|| "-".to_string());
                        view! { <tr class="border-b border-border last:border-b-0"><td class="px-4 py-3 text-muted-foreground">{account.id}</td><td class="px-4 py-3 font-medium text-foreground"><a class="hover:text-primary" href=format!("/admin/accounts/{}", account.id)>{account.username}</a></td><td class="px-4 py-3 text-muted-foreground">{email}</td><td class="px-4 py-3">{account.gmlevel}</td><td class="px-4 py-3">{state}</td></tr> }
                    }).collect_view()}</tbody>
                </table>
            </div>
            <div class="mt-4 flex items-center gap-3 text-xs">
                <button class="border border-input px-3 py-2 disabled:opacity-50" disabled=current_page == 1 on:click=move |_| page.update(|value| *value = value.saturating_sub(1))>"Previous"</button>
                <span class="text-muted-foreground">"Page " {current_page} " | " {result.total} " accounts"</span>
                <button class="border border-input px-3 py-2 disabled:opacity-50" disabled=!has_next on:click=move |_| page.update(|value| *value += 1)>"Next"</button>
            </div>
        }.into_any()
        }
        Err(error) => {
            view! { <p class="mt-8 text-xs text-destructive">{error.to_string()}</p> }.into_any()
        }
    }
}

#[component]
pub fn AdminAccountDetail() -> impl IntoView {
    let params = use_params_map();
    let account = Resource::new(
        move || {
            params
                .read()
                .get("account_id")
                .and_then(|id| id.parse::<u32>().ok())
        },
        |id| async move {
            match id {
                Some(id) => portal::get_admin_account(id).await,
                None => Ok(None),
            }
        },
    );
    let realm_access = Resource::new(
        move || {
            params
                .read()
                .get("account_id")
                .and_then(|id| id.parse::<u32>().ok())
        },
        |id| async move {
            match id {
                Some(id) => portal::get_admin_account_realm_access(id).await,
                None => Ok(Vec::new()),
            }
        },
    );
    let characters = Resource::new(
        move || {
            params
                .read()
                .get("account_id")
                .and_then(|id| id.parse::<u32>().ok())
        },
        |id| async move {
            match id {
                Some(id) => portal::get_admin_account_characters(id).await,
                None => Ok(Vec::new()),
            }
        },
    );
    let sessions = Resource::new(
        move || {
            params
                .read()
                .get("account_id")
                .and_then(|id| id.parse::<u32>().ok())
        },
        |id| async move {
            match id {
                Some(id) => portal::get_admin_account_sessions(id).await,
                None => Ok(Vec::new()),
            }
        },
    );
    let audit = Resource::new(
        move || {
            params
                .read()
                .get("account_id")
                .and_then(|id| id.parse::<u32>().ok())
        },
        |id| async move { portal::get_admin_audit_log(id).await },
    );
    view! {
        <AdminShell active=AdminSection::Accounts>
            <a class="text-xs font-semibold uppercase tracking-[0.3em] text-primary" href="/admin/accounts">"Account management"</a>
            <Suspense fallback=move || view! { <p class="mt-8 text-xs text-muted-foreground">"Loading account..."</p> }>
                {move || account.get().map(render_admin_account_detail)}
            </Suspense>
            <div class="mt-8 grid gap-4 xl:grid-cols-2">
            <section class="border border-border bg-card p-5">
                <p class="text-xs font-medium text-foreground">"Realm-specific roles"</p>
                <Suspense fallback=move || view! { <p class="mt-3 text-xs text-muted-foreground">"Loading realm roles..."</p> }>
                    {move || realm_access.get().map(render_realm_access)}
                </Suspense>
            </section>
            <section class="border border-border bg-card p-5"><p class="text-xs font-medium text-foreground">"Characters"</p><Suspense fallback=move || view! { <p class="mt-3 text-xs text-muted-foreground">"Loading characters..."</p> }>{move || characters.get().map(render_admin_characters)}</Suspense></section>
            <section class="border border-border bg-card p-5"><p class="text-xs font-medium text-foreground">"Portal sessions"</p><Suspense fallback=move || view! { <p class="mt-3 text-xs text-muted-foreground">"Loading sessions..."</p> }>{move || sessions.get().map(render_admin_sessions)}</Suspense></section>
            <section class="border border-border bg-card p-5 xl:col-span-2"><p class="text-xs font-medium text-foreground">"Account audit history"</p><Suspense fallback=move || view! { <p class="mt-3 text-xs text-muted-foreground">"Loading audit history..."</p> }>{move || audit.get().map(super::render_audit_entries)}</Suspense></section>
            </div>
        </AdminShell>
    }
}

fn render_admin_characters(result: Result<Vec<portal::AdminCharacter>, ServerFnError>) -> AnyView {
    match result {
        Ok(characters) if characters.is_empty() => view! { <p class="mt-3 text-xs text-muted-foreground">"No characters."</p> }.into_any(),
        Ok(characters) => view! { <ul class="mt-3 divide-y divide-border text-xs">{characters.into_iter().map(|character| view! { <li class="flex justify-between py-2"><span>{character.name} " (" {character.guid} ")"</span><span>{"Level "} {character.level} {if character.online != 0 { " online" } else { " offline" }}</span></li> }).collect_view()}</ul> }.into_any(),
        Err(error) => view! { <p class="mt-3 text-xs text-destructive">{error.to_string()}</p> }.into_any(),
    }
}

fn render_admin_sessions(result: Result<Vec<portal::AdminSession>, ServerFnError>) -> AnyView {
    match result {
        Ok(sessions) if sessions.is_empty() => view! { <p class="mt-3 text-xs text-muted-foreground">"No portal sessions."</p> }.into_any(),
        Ok(sessions) => view! { <ul class="mt-3 divide-y divide-border text-xs">{sessions.into_iter().map(|session| { let action = format!("/admin/accounts/{}/sessions/revoke", session.account_id); view! { <li class="flex items-center justify-between gap-3 py-2"><span class="text-muted-foreground">"Created " {session.created_at} " | last seen " {session.last_seen_at} " | expires " {session.expires_at}</span><form action=action method="post"><input name="session_id" type="hidden" value=session.id /><button class="text-destructive hover:underline" type="submit">"Revoke"</button></form></li> } }).collect_view()}</ul> }.into_any(),
        Err(error) => view! { <p class="mt-3 text-xs text-destructive">{error.to_string()}</p> }.into_any(),
    }
}

fn render_realm_access(result: Result<Vec<portal::RealmAccess>, ServerFnError>) -> AnyView {
    match result {
        Ok(access) if access.is_empty() => view! { <p class="mt-3 text-xs text-muted-foreground">"No realm-specific role assignments."</p> }.into_any(),
        Ok(access) => view! { <ul class="mt-3 divide-y divide-border text-xs">{access.into_iter().map(|entry| view! { <li class="flex justify-between py-2"><span>"Realm " {entry.realm_id}</span><span>{entry.gmlevel} " " {security_level_name(entry.gmlevel)}</span></li> }).collect_view()}</ul> }.into_any(),
        Err(error) => view! { <p class="mt-3 text-xs text-destructive">{error.to_string()}</p> }.into_any(),
    }
}

fn render_admin_account_detail(
    result: Result<Option<portal::AdminAccountDetail>, ServerFnError>,
) -> AnyView {
    match result {
        Ok(Some(account)) => {
            let email_input = account.email.clone().unwrap_or_default();
            let email = account
                .email
                .unwrap_or_else(|| "No email address".to_string());
            let state = if account.banned != 0 {
                "Banned"
            } else if account.locked != 0 {
                "Locked"
            } else {
                "Active"
            };
            let action = format!("/admin/accounts/{}", account.id);
            view! {
                <h1 class="mt-5 font-sans text-3xl font-semibold tracking-tight">{account.username}</h1>
                <dl class="mt-8 divide-y divide-border border-y border-border text-xs">
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Account ID"</dt><dd>{account.id}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Email"</dt><dd>{email}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Global security level"</dt><dd>{account.gmlevel}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"State"</dt><dd>{state}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Last IP"</dt><dd>{account.last_ip}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Expansion"</dt><dd>{account.expansion}</dd></div>
                </dl>
                <form class="mt-8 space-y-5 border-t border-border pt-6 text-xs" action=action method="post">
                    <p class="font-medium text-foreground">"Edit account"</p>
                    <label class="block text-muted-foreground" for="admin-email">"Email"<input class="mt-2 block w-full border border-input bg-input px-3 py-2 text-foreground" id="admin-email" name="email" type="email" value=email_input /></label>
                    <label class="block text-muted-foreground" for="admin-gmlevel">"Global security level"
                        <select class="mt-2 block border border-input bg-input px-3 py-2 text-foreground" id="admin-gmlevel" name="gmlevel">
                            {(0_u8..=7).map(|level| view! { <option value=level selected=account.gmlevel == level>{level} " " {security_level_name(level)}</option> }).collect_view()}
                        </select>
                    </label>
                    <label class="flex items-center gap-2 text-muted-foreground"><input name="locked" type="checkbox" value="true" checked=account.locked != 0 />"Locked"</label>
                    <Button button_type="submit">"Save changes"</Button>
                </form>
                <section class="mt-8 grid gap-6 border-t border-border pt-6 text-xs sm:grid-cols-2">
                    <form class="space-y-3" action=format!("/admin/accounts/{}/ban", account.id) method="post"><p class="font-medium">"Ban"</p><label class="flex gap-2"><input name="active" type="checkbox" value="true" checked=account.banned != 0 />"Active"</label><input class="w-full border border-input bg-input px-3 py-2" name="duration_seconds" type="number" min="0" placeholder="Seconds, 0 = permanent"/><input class="w-full border border-input bg-input px-3 py-2" name="reason" maxlength="255" placeholder="Reason"/><Button button_type="submit">"Update ban"</Button></form>
                    <form class="space-y-3" action=format!("/admin/accounts/{}/mute", account.id) method="post"><p class="font-medium">"Mute"</p><label class="flex gap-2"><input name="active" type="checkbox" value="true" checked={account.muted_until > 0} />"Active"</label><input class="w-full border border-input bg-input px-3 py-2" name="duration_seconds" type="number" min="1" placeholder="Duration in seconds"/><input class="w-full border border-input bg-input px-3 py-2" name="reason" maxlength="255" placeholder="Reason"/><Button button_type="submit">"Update mute"</Button></form>
                </section>
                <form class="mt-8 space-y-3 border-t border-border pt-6 text-xs" action=format!("/admin/accounts/{}/realm-role", account.id) method="post"><p class="font-medium">"Set realm-specific role"</p><div class="flex gap-3"><input class="w-32 border border-input bg-input px-3 py-2" name="realm_id" type="number" placeholder="Realm ID" required/><select class="border border-input bg-input px-3 py-2" name="gmlevel">{(0_u8..=7).map(|level| view! { <option value=level>{level} " " {security_level_name(level)}</option> }).collect_view()}</select><Button button_type="submit">"Save role"</Button></div><p class="text-muted-foreground">"Set level 0 to remove the role."</p></form>
            }.into_any()
        }
        Ok(None) => {
            view! { <p class="mt-8 text-xs text-muted-foreground">"Account not found."</p> }
                .into_any()
        }
        Err(error) => {
            view! { <p class="mt-8 text-xs text-destructive">{error.to_string()}</p> }.into_any()
        }
    }
}

fn security_level_name(level: u8) -> &'static str {
    portal::SECURITY_LEVELS
        .iter()
        .find(|(value, _)| *value == level)
        .map(|(_, name)| *name)
        .unwrap_or("Unknown")
}
