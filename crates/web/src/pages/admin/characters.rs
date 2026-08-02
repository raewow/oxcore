use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::pages::admin::shell::{AdminSection, AdminShell};
use crate::portal;

#[component]
pub fn AdminCharacters() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let page = RwSignal::new(1_u32);
    let characters = Resource::new(
        move || (search.get(), page.get()),
        |(search, page)| portal::get_admin_characters(Some(search), page),
    );
    view! {
        <AdminShell active=AdminSection::Characters>
            <p class="text-xs font-semibold uppercase tracking-[0.3em] text-primary">"Character management"</p>
            <h1 class="mt-4 font-sans text-3xl font-semibold tracking-tight">"Characters"</h1>
            <p class="mt-3 text-xs text-muted-foreground">"Search characters by name or ID."</p>
            <label class="mt-6 block max-w-md text-xs text-muted-foreground" for="character-search">"Search"
                <input class="mt-2 block w-full border border-input bg-input px-3 py-2 text-foreground" id="character-search" type="search" placeholder="Character name or ID" on:input=move |event| { search.set(event_target_value(&event)); page.set(1); } />
            </label>
            <Suspense fallback=move || view! { <p class="mt-8 text-xs text-muted-foreground">"Loading characters..."</p> }>
                {move || characters.get().map(move |result| render_admin_characters(result, page))}
            </Suspense>
        </AdminShell>
    }
}

fn render_admin_characters(
    result: Result<portal::AdminCharacterPage, ServerFnError>,
    page: RwSignal<u32>,
) -> AnyView {
    match result {
        Ok(result) => {
            let has_next = result.page as u64 * (result.page_size as u64) < result.total;
            let current_page = result.page;
            let characters = result.characters;
            view! {
                <div class="mt-8 overflow-x-auto border border-border">
                    <table class="min-w-full text-left text-xs">
                        <thead class="border-b border-border bg-card text-muted-foreground"><tr><th class="px-4 py-3">"Character"</th><th class="px-4 py-3">"Level"</th><th class="px-4 py-3">"Class"</th><th class="px-4 py-3">"Race"</th><th class="px-4 py-3">"State"</th></tr></thead>
                        <tbody>{characters.into_iter().map(|character| {
                            let state = if character.online != 0 { "Online" } else { "Offline" };
                            view! { <tr class="border-b border-border last:border-b-0"><td class="px-4 py-3 font-medium text-foreground"><a class="hover:text-primary" href=format!("/admin/characters/{}", character.guid)>{character.name}</a></td><td class="px-4 py-3 text-muted-foreground">{character.level}</td><td class="px-4 py-3">{portal::class_name(character.class)}</td><td class="px-4 py-3">{portal::race_name(character.race)}</td><td class="px-4 py-3">{state}</td></tr> }
                        }).collect_view()}</tbody>
                    </table>
                </div>
                <div class="mt-4 flex items-center gap-3 text-xs">
                    <button class="border border-input px-3 py-2 disabled:opacity-50" disabled=current_page == 1 on:click=move |_| page.update(|value| *value = value.saturating_sub(1))>"Previous"</button>
                    <span class="text-muted-foreground">"Page " {current_page} " | " {result.total} " characters"</span>
                    <button class="border border-input px-3 py-2 disabled:opacity-50" disabled=!has_next on:click=move |_| page.update(|value| *value += 1)>"Next"</button>
                </div>
            }
            .into_any()
        }
        Err(error) => {
            view! { <p class="mt-8 text-xs text-destructive">{error.to_string()}</p> }.into_any()
        }
    }
}

#[component]
pub fn AdminCharacterDetail() -> impl IntoView {
    let params = use_params_map();
    let character = Resource::new(
        move || {
            params
                .read()
                .get("guid")
                .and_then(|guid| guid.parse::<u32>().ok())
        },
        |guid| async move {
            match guid {
                Some(guid) => portal::get_admin_character_detail(guid).await,
                None => Ok(None),
            }
        },
    );
    view! {
        <AdminShell active=AdminSection::Characters>
            <a class="text-xs font-semibold uppercase tracking-[0.3em] text-primary" href="/admin/characters">"Character management"</a>
            <Suspense fallback=move || view! { <p class="mt-8 text-xs text-muted-foreground">"Loading character..."</p> }>
                {move || character.get().map(render_admin_character_detail)}
            </Suspense>
        </AdminShell>
    }
}

fn render_admin_character_detail(
    result: Result<Option<portal::AdminCharacterDetail>, ServerFnError>,
) -> AnyView {
    match result {
        Ok(Some(character)) => {
            let state = if character.online != 0 {
                "Online"
            } else {
                "Offline"
            };
            let account_username = character
                .account_username
                .clone()
                .unwrap_or_else(|| "Unknown account".to_string());
            let position = format!(
                "{:.1} / {:.1} / {:.1}",
                character.position_x, character.position_y, character.position_z
            );
            view! {
                <h1 class="mt-5 font-sans text-3xl font-semibold tracking-tight">{character.name}</h1>
                <p class="mt-3 text-xs text-muted-foreground">"Character " {character.guid}</p>
                <dl class="mt-8 divide-y divide-border border-y border-border text-xs">
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Account"</dt><dd><a class="text-primary hover:underline" href=format!("/admin/accounts/{}", character.account)>{account_username} " (" {character.account} ")"</a></dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Level"</dt><dd>{character.level}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Class"</dt><dd>{portal::class_name(character.class)}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Race"</dt><dd>{portal::race_name(character.race)}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Gender"</dt><dd>{portal::gender_name(character.gender)}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Status"</dt><dd>{state}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Zone"</dt><dd>{character.zone}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Map"</dt><dd>{character.map}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Position"</dt><dd>{position}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Money"</dt><dd>{portal::format_money(character.money)}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Played time"</dt><dd>{portal::format_played_time(character.played_time_total)}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Created"</dt><dd>{portal::format_timestamp(character.create_time)}</dd></div>
                    <div class="flex justify-between gap-6 py-3"><dt class="text-muted-foreground">"Last logout"</dt><dd>{portal::format_timestamp(character.logout_time)}</dd></div>
                </dl>
                <a class="mt-6 inline-block text-xs text-primary hover:underline" href="/admin/accounts">"Return to accounts"</a>
            }
            .into_any()
        }
        Ok(None) => {
            view! { <p class="mt-8 text-xs text-muted-foreground">"Character not found."</p> }
                .into_any()
        }
        Err(error) => {
            view! { <p class="mt-8 text-xs text-destructive">{error.to_string()}</p> }.into_any()
        }
    }
}
