use leptos::prelude::*;
use leptos_meta::Script;

use crate::pages::admin::shell::{AdminSection, AdminShell};
use crate::portal;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChatSelection {
    All,
    Live,
    Conversation(String, Option<String>),
    Player(String),
}

#[derive(Debug, Clone)]
struct ModerationTarget {
    name: String,
    account: Option<u32>,
}

#[component]
pub fn AdminChat() -> impl IntoView {
    let selected = RwSignal::new(ChatSelection::All);
    let player_search = RwSignal::new(String::new());
    let whisper_target = RwSignal::new(String::new());
    let start_chat_open = RwSignal::new(false);
    let moderation_target = RwSignal::new(None::<ModerationTarget>);
    let overview = Resource::new(|| (), |_| portal::get_chat_overview());
    let participants = Resource::new(
        move || player_search.get(),
        |search| portal::get_chat_participants(Some(search)),
    );

    view! {
        <AdminShell active=AdminSection::Chat>
            <div class="mx-auto flex min-h-[calc(100vh-5rem)] max-w-[1500px] flex-col overflow-hidden border border-border bg-card shadow-2xl shadow-black/20 lg:flex-row">
                <aside class="flex w-full shrink-0 flex-col border-b border-border bg-background/70 lg:w-72 lg:border-r lg:border-b-0">
                    <div class="border-b border-border p-3">
                        <div class="flex items-center gap-2">
                            <input
                                class="h-9 min-w-0 flex-1 bg-input px-3 text-xs text-foreground outline-none placeholder:text-muted-foreground focus:ring-1 focus:ring-ring"
                                type="search"
                                placeholder="Search players and chats"
                                aria-label="Search players and chats"
                                on:input=move |event| player_search.set(event_target_value(&event))
                            />
                            <button
                                class="h-9 shrink-0 bg-primary px-3 text-xs font-semibold text-primary-foreground hover:bg-primary/85"
                                type="button"
                                on:click=move |_| start_chat_open.update(|open| *open = !*open)
                            >
                                "New chat"
                            </button>
                        </div>
                        {move || start_chat_open.get().then(|| view! {
                            <div class="mt-2 border border-primary/35 bg-card p-2 text-xs text-muted-foreground">
                                "Search for a player, then select their name to start a Whisper."
                            </div>
                        })}
                    </div>

                    <nav class="flex gap-1 border-b border-border p-2 lg:block" aria-label="Chat views">
                        <SidebarEntry label="All messages" active=move || selected.get() == ChatSelection::All on_select=move || selected.set(ChatSelection::All) />
                        <SidebarEntry label="Live now" active=move || selected.get() == ChatSelection::Live on_select=move || selected.set(ChatSelection::Live) />
                    </nav>

                    <div class="min-h-0 flex-1 overflow-y-auto p-2">
                        <p class="px-2 py-2 text-[10px] font-bold uppercase tracking-[0.16em] text-muted-foreground">"Players"</p>
                        <Suspense fallback=move || view! { <p class="px-2 py-1 text-xs text-muted-foreground">"Loading players..."</p> }>
                            {move || participants.get().map(move |result| render_participants(result, selected, whisper_target, start_chat_open))}
                        </Suspense>

                        <p class="mt-3 px-2 py-2 text-[10px] font-bold uppercase tracking-[0.16em] text-muted-foreground">"Groups and channels"</p>
                        <Suspense fallback=move || view! { <p class="px-2 py-1 text-xs text-muted-foreground">"Loading conversations..."</p> }>
                            {move || overview.get().map(move |result| render_conversations(result, selected, player_search.get()))}
                        </Suspense>
                    </div>
                </aside>

                <section class="flex min-h-[65vh] min-w-0 flex-1 flex-col bg-card">
                    <ChatHeader selected=selected />
                    <main class="min-h-0 flex-1 overflow-y-auto px-4 py-3 sm:px-6">
                        <ChatBody selected=selected moderation_target=moderation_target />
                    </main>
                    <ChatComposer target=whisper_target />
                </section>
            </div>
            <ModerationPopover target=moderation_target />
        </AdminShell>
    }
}

#[component]
fn SidebarEntry(
    label: &'static str,
    active: impl Fn() -> bool + Send + Sync + 'static,
    on_select: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let class = move || {
        if active() {
            "flex-1 px-3 py-2 text-left text-xs font-semibold text-primary bg-primary/10 lg:block lg:w-full"
        } else {
            "flex-1 px-3 py-2 text-left text-xs text-muted-foreground hover:bg-input hover:text-foreground lg:block lg:w-full"
        }
    };
    view! { <button class=class type="button" on:click=move |_| on_select()>{label}</button> }
}

fn render_participants(
    result: Result<Vec<portal::ChatParticipant>, ServerFnError>,
    selected: RwSignal<ChatSelection>,
    whisper_target: RwSignal<String>,
    start_chat_open: RwSignal<bool>,
) -> AnyView {
    match result {
        Ok(players) if players.is_empty() => view! { <p class="px-2 py-1 text-xs text-muted-foreground">"No matching players."</p> }.into_any(),
        Ok(players) => view! {
            <ul class="space-y-0.5">
                {players.into_iter().map(move |player| {
                    let name = player.name.clone();
                    let current_name = name.clone();
                    let class = move || if selected.get() == ChatSelection::Player(current_name.clone()) {
                        "flex w-full items-center gap-2 bg-primary/10 px-2 py-2 text-left text-primary"
                    } else {
                        "flex w-full items-center gap-2 px-2 py-2 text-left text-muted-foreground hover:bg-input hover:text-foreground"
                    };
                    view! {
                        <li>
                            <button class=class type="button" on:click=move |_| {
                                whisper_target.set(name.clone());
                                selected.set(ChatSelection::Player(name.clone()));
                                start_chat_open.set(false);
                            }>
                                <span class="grid h-6 w-6 shrink-0 place-items-center bg-input text-[10px] font-bold text-primary">{player.name.chars().next().unwrap_or('?')}</span>
                                <span class="min-w-0 flex-1 truncate text-xs">{player.name.clone()}</span>
                                <span class="text-[10px] text-muted-foreground">{player.message_count}</span>
                            </button>
                        </li>
                    }
                }).collect_view()}
            </ul>
        }.into_any(),
        Err(error) => view! { <p class="px-2 py-1 text-xs text-destructive">{error.to_string()}</p> }.into_any(),
    }
}

fn render_conversations(
    result: Result<portal::ChatOverview, ServerFnError>,
    selected: RwSignal<ChatSelection>,
    search: String,
) -> AnyView {
    match result {
        Ok(overview) if overview.channels.is_empty() => {
            view! { <p class="px-2 py-1 text-xs text-muted-foreground">"No chat logged yet."</p> }
                .into_any()
        }
        Ok(overview) => {
            let search = search.to_lowercase();
            let channels = overview
                .channels
                .into_iter()
                .filter(|channel| {
                    search.is_empty()
                        || channel.channel_type.to_lowercase().contains(&search)
                        || channel
                            .channel_name
                            .as_deref()
                            .unwrap_or_default()
                            .to_lowercase()
                            .contains(&search)
                })
                .collect::<Vec<_>>();
            if channels.is_empty() {
                return view! { <p class="px-2 py-1 text-xs text-muted-foreground">"No matching conversations."</p> }.into_any();
            }
            view! {
            <ul class="space-y-0.5">
                {channels.into_iter().map(move |channel| {
                    let channel_type = channel.channel_type.clone();
                    let channel_name = channel.channel_name.clone();
                    let label = channel_name.clone().unwrap_or_else(|| channel_type.clone());
                    let type_label = channel_type.clone();
                    let active_type = channel_type.clone();
                    let active_name = channel_name.clone();
                    let class = move || if selected.get() == ChatSelection::Conversation(active_type.clone(), active_name.clone()) {
                        "flex w-full items-center gap-2 bg-primary/10 px-2 py-2 text-left text-primary"
                    } else {
                        "flex w-full items-center gap-2 px-2 py-2 text-left text-muted-foreground hover:bg-input hover:text-foreground"
                    };
                    view! {
                        <li>
                            <button class=class type="button" on:click=move |_| selected.set(ChatSelection::Conversation(channel_type.clone(), channel_name.clone()))>
                                <span class="text-primary/80">"#"</span>
                                <span class="min-w-0 flex-1 truncate text-xs">{label}</span>
                                <span class="text-[10px] text-muted-foreground">{type_label}</span>
                            </button>
                        </li>
                    }
                }).collect_view()}
            </ul>
            }.into_any()
        }
        Err(error) => {
            view! { <p class="px-2 py-1 text-xs text-destructive">{error.to_string()}</p> }
                .into_any()
        }
    }
}

#[component]
fn ChatHeader(selected: RwSignal<ChatSelection>) -> impl IntoView {
    view! {
        <header class="flex h-14 shrink-0 items-center border-b border-border px-4 sm:px-6">
            {move || {
                let (label, detail) = match selected.get() {
                    ChatSelection::All => ("All messages".to_string(), "Realm chat archive".to_string()),
                    ChatSelection::Live => ("Live now".to_string(), "Refreshing every 1.5 seconds".to_string()),
                    ChatSelection::Conversation(channel_type, channel_name) => (channel_name.unwrap_or(channel_type.clone()), channel_type),
                    ChatSelection::Player(name) => (name, "Player activity and Whisper target".to_string()),
                };
                view! {
                    <span class="mr-3 text-lg text-primary">"#"</span>
                    <div class="min-w-0">
                        <h1 class="truncate text-sm font-semibold text-foreground">{label}</h1>
                        <p class="truncate text-[11px] text-muted-foreground">{detail}</p>
                    </div>
                }
            }}
        </header>
    }
}

#[component]
fn ChatBody(
    selected: RwSignal<ChatSelection>,
    moderation_target: RwSignal<Option<ModerationTarget>>,
) -> impl IntoView {
    let conversation = Resource::new(
        move || selected.get(),
        |selection| async move {
            match selection {
                ChatSelection::Conversation(channel_type, channel_name) => {
                    portal::get_chat_channel(channel_type, channel_name, 0, 200).await
                }
                ChatSelection::Player(name) => portal::get_player_chat(name)
                    .await
                    .map(|detail| detail.messages),
                ChatSelection::All | ChatSelection::Live => Ok(Vec::new()),
            }
        },
    );

    view! {
        {move || match selected.get() {
            ChatSelection::All | ChatSelection::Live => view! { <LiveFeed /> }.into_any(),
            ChatSelection::Conversation(_, _) | ChatSelection::Player(_) => view! {
                <Suspense fallback=move || view! { <p class="py-8 text-center text-xs text-muted-foreground">"Loading messages..."</p> }>
                    {move || conversation.get().map(move |result| render_messages(result, moderation_target))}
                </Suspense>
            }.into_any(),
        }}
    }
}

#[component]
fn LiveFeed() -> impl IntoView {
    view! {
        <div id="chat-live-feed" class="space-y-1"></div>
        <Script defer="defer" src="/chat-live.js" />
    }
}

fn render_messages(
    result: Result<Vec<portal::ChatMessage>, ServerFnError>,
    moderation_target: RwSignal<Option<ModerationTarget>>,
) -> AnyView {
    match result {
        Ok(messages) if messages.is_empty() => view! { <p class="py-8 text-center text-xs text-muted-foreground">"No messages in this conversation."</p> }.into_any(),
        Ok(messages) => render_message_list(messages, moderation_target),
        Err(error) => view! { <p class="py-8 text-center text-xs text-destructive">{error.to_string()}</p> }.into_any(),
    }
}

fn render_message_list(
    messages: Vec<portal::ChatMessage>,
    moderation_target: RwSignal<Option<ModerationTarget>>,
) -> AnyView {
    messages.into_iter().map(move |message| {
        let sender = message.sender_name.clone().unwrap_or_else(|| "Unknown".to_string());
        let target = message.target_name.clone();
        let channel = message.channel_name.clone();
        let type_label = message.channel_type.clone();
        let time = message.time;
        let text = message.message.clone();
        let sender_for_popover = sender.clone();
        let account = message.sender_account;
        view! {
            <article class="group flex gap-3 border-b border-border/50 py-3 last:border-b-0">
                <span class="grid h-8 w-8 shrink-0 place-items-center bg-input text-xs font-bold text-primary">{sender.chars().next().unwrap_or('?')}</span>
                <div class="min-w-0 flex-1">
                    <div class="flex flex-wrap items-baseline gap-x-2">
                        <button class="font-semibold text-foreground hover:text-primary" type="button" on:click=move |_| moderation_target.set(Some(ModerationTarget { name: sender_for_popover.clone(), account }))>{sender}</button>
                        <span class="text-[10px] text-muted-foreground">{time}</span>
                        <span class="text-[10px] uppercase tracking-wide text-primary/80">{type_label}</span>
                        {channel.map(|channel| view! { <span class="text-[10px] text-muted-foreground">{"# "}{channel}</span> })}
                    </div>
                    {target.map(move |target| {
                        let target_for_popover = target.clone();
                        view! { <p class="mt-0.5 text-[11px] text-muted-foreground">"to " <button class="hover:text-primary" type="button" on:click=move |_| moderation_target.set(Some(ModerationTarget { name: target_for_popover.clone(), account: None }))>{target}</button></p> }
                    })}
                    <p class="mt-1 break-words text-sm leading-6 text-foreground">{text}</p>
                </div>
            </article>
        }
    }).collect_view().into_any()
}

#[component]
fn ChatComposer(target: RwSignal<String>) -> impl IntoView {
    let gm_chars = Resource::new(|| (), |_| portal::get_gm_characters());
    let sender_guid = RwSignal::new(0_u32);
    let message = RwSignal::new(String::new());
    let send_error = RwSignal::new(None::<String>);
    let send_trigger = RwSignal::new(0_u64);
    let send_result = Resource::new(
        move || send_trigger.get(),
        move |_| async move {
            portal::send_chat_message(
                sender_guid.get(),
                "Whisper".to_string(),
                None,
                Some(target.get()),
                message.get(),
            )
            .await
        },
    );
    Effect::new(move |_| {
        if let Some(Ok(result)) = send_result.get() {
            if result.accepted {
                message.set(String::new());
            }
        }
    });
    let send = move |_| {
        if sender_guid.get() == 0 {
            send_error.set(Some("Select an online sender character".to_string()));
            return;
        }
        if target.get().trim().is_empty() {
            send_error.set(Some("Select a player to Whisper".to_string()));
            return;
        }
        if message.get().trim().is_empty() {
            send_error.set(Some("Message is empty".to_string()));
            return;
        }
        send_error.set(None);
        send_trigger.update(|value| *value += 1);
    };
    view! {
        <footer class="shrink-0 border-t border-border bg-background/50 p-3 sm:p-4">
            <div class="mb-2 flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
                <span>"Whispering"</span>
                <input class="h-7 w-36 bg-input px-2 text-xs text-foreground outline-none focus:ring-1 focus:ring-ring" placeholder="Player name" prop:value=move || target.get() on:input=move |event| target.set(event_target_value(&event)) />
                <select class="h-7 max-w-48 bg-input px-2 text-xs text-foreground outline-none focus:ring-1 focus:ring-ring" on:change=move |event| sender_guid.set(event_target_value(&event).parse().unwrap_or(0))>
                    <option value="0">"Send as..."</option>
                    {move || match gm_chars.get() {
                        Some(Ok(characters)) => characters.into_iter().map(|character| view! { <option value=character.guid>{character.name} " (L" {character.level} ")"</option> }).collect_view().into_any(),
                        Some(Err(error)) => view! { <option value="0">{error.to_string()}</option> }.into_any(),
                        None => view! { <option value="0">"Loading characters..."</option> }.into_any(),
                    }}
                </select>
            </div>
            <div class="flex gap-2">
                <input class="h-10 min-w-0 flex-1 bg-input px-3 text-sm text-foreground outline-none placeholder:text-muted-foreground focus:ring-1 focus:ring-ring" maxlength="512" placeholder=move || { let target_name = target.get(); if target_name.is_empty() { "Message a player".to_string() } else { format!("Message {target_name}") } } prop:value=move || message.get() on:input=move |event| message.set(event_target_value(&event)) />
                <button class="h-10 bg-primary px-4 text-xs font-semibold text-primary-foreground hover:bg-primary/85" type="button" on:click=send>"Send"</button>
            </div>
            {move || send_error.get().map(|error| view! { <p class="mt-2 text-xs text-destructive">{error}</p> })}
            {move || match send_result.get() { Some(Ok(result)) => view! { <p class="mt-2 text-xs text-muted-foreground">{result.note}</p> }.into_any(), Some(Err(error)) => view! { <p class="mt-2 text-xs text-destructive">{error.to_string()}</p> }.into_any(), None => ().into_any() }}
        </footer>
    }
}

#[component]
fn ModerationPopover(target: RwSignal<Option<ModerationTarget>>) -> impl IntoView {
    view! {
        {move || target.get().map(|selected_target| {
            let name = selected_target.name.clone();
            view! {
                <div class="fixed bottom-5 right-5 z-50 w-52 border border-primary/30 bg-card p-3 shadow-xl shadow-black/40" role="dialog" aria-label="Quick moderation actions">
                    <div class="flex items-start justify-between gap-3">
                        <div><p class="text-[10px] font-bold uppercase tracking-[0.15em] text-muted-foreground">"Quick actions"</p><p class="mt-1 text-sm font-semibold text-foreground">{name}</p></div>
                        <button class="text-muted-foreground hover:text-foreground" type="button" aria-label="Close moderation actions" on:click=move |_| target.set(None)>"×"</button>
                    </div>
                    {selected_target.account.map(|account| view! { <p class="mt-1 text-[10px] text-muted-foreground">{format!("Account #{account}")}</p> })}
                    <div class="mt-3 grid grid-cols-2 gap-1">
                        {['M', 'K', 'B', 'V'].into_iter().zip(["Mute", "Kick", "Ban", "View account"]).map(|(icon, label)| view! { <button class="border border-input px-2 py-2 text-left text-[11px] text-muted-foreground hover:border-primary/50 hover:bg-input hover:text-foreground" type="button"><span class="mr-1 text-primary">{icon}</span>{label}</button> }).collect_view()}
                    </div>
                    <p class="mt-2 text-[10px] text-muted-foreground">"Actions are not enabled yet."</p>
                </div>
            }
        })}
    }
}
