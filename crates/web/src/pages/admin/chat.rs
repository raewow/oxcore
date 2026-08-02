use leptos::prelude::*;
use leptos_meta::Script;

use crate::components::ui::Button;
use crate::pages::admin::shell::{AdminSection, AdminShell};
use crate::portal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatTab {
    Live,
    Channels,
    Players,
}

#[component]
pub fn AdminChat() -> impl IntoView {
    let tab = RwSignal::new(ChatTab::Live);

    view! {
        <AdminShell active=AdminSection::Chat>
            <p class="text-xs font-semibold uppercase tracking-[0.3em] text-primary">"Chat"</p>
            <h1 class="mt-4 font-sans text-3xl font-semibold tracking-tight">"Chat monitoring"</h1>
            <p class="mt-3 text-xs text-muted-foreground">"A live view of every chat message on the realm, conversation history, per-player search, and sending messages to players as a GM. All messages are logged by the world server."</p>

            <GmComposer />

            <div class="mt-6 flex items-center gap-2 text-xs">
                {[(ChatTab::Live, "Live feed"), (ChatTab::Channels, "Channels"), (ChatTab::Players, "Players")]
                    .into_iter()
                    .map(|(value, label)| {
                        let is_active = move || tab.get() == value;
                        let class = move || {
                            if is_active() {
                                "border border-primary/40 bg-primary/10 px-3 py-2 font-medium text-primary"
                            } else {
                                "border border-input px-3 py-2 text-muted-foreground hover:bg-input hover:text-foreground"
                            }
                        };
                        view! { <button class=class on:click=move |_| tab.set(value)>{label}</button> }
                    })
                    .collect_view()}
            </div>

            <div class="mt-4">
                {move || match tab.get() {
                    ChatTab::Live => view! { <LiveFeed /> }.into_any(),
                    ChatTab::Channels => view! { <ChannelsBrowser /> }.into_any(),
                    ChatTab::Players => view! { <PlayersBrowser /> }.into_any(),
                }}
            </div>
        </AdminShell>
    }
}

/// Send a chat message as one of the acting GM's online characters.
#[component]
fn GmComposer() -> impl IntoView {
    let gm_chars = Resource::new(|| (), |_| portal::get_gm_characters());
    let sender_guid = RwSignal::new(0_u32);
    let chat_type = RwSignal::new("Whisper".to_string());
    let target = RwSignal::new(String::new());
    let channel = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());
    let send_error = RwSignal::new(None::<String>);
    let send_trigger = RwSignal::new(0_u64);

    let send_result = Resource::new(
        move || send_trigger.get(),
        move |_| {
            let guid = sender_guid.get();
            let chat_type = chat_type.get();
            let channel = channel.get();
            let target = target.get();
            let message = message.get();
            async move {
                let channel_param = if chat_type == "Channel" {
                    Some(channel)
                } else {
                    None
                };
                portal::send_chat_message(guid, chat_type, channel_param, Some(target), message)
                    .await
            }
        },
    );

    Effect::new(move |_| {
        if let Some(Ok(result)) = send_result.get() {
            if result.accepted {
                message.set(String::new());
            }
        }
    });

    let do_send = move |_| {
        if sender_guid.get() == 0 {
            send_error.set(Some("Select an online sender character".to_string()));
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
        <section class="mt-6 border border-border bg-card px-4 py-4 text-xs">
            <p class="font-medium text-foreground">"Send a message as a GM"</p>
            <p class="mt-2 text-muted-foreground">"Choose one of your online characters to speak through. Whisper needs a target player, Say is said in your current map, Channel needs a channel name."</p>
            <div class="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-4">
                <label class="block text-muted-foreground" for="gm-sender">"Sender character"
                    <select class="mt-2 block w-full border border-input bg-input px-2.5 py-2 text-foreground" id="gm-sender" on:change=move |event| sender_guid.set(event_target_value(&event).parse::<u32>().unwrap_or(0))>
                        <option value="0">"Loading characters..."</option>
                        {move || {
                            match gm_chars.get() {
                                None => view! {}.into_any(),
                                Some(Ok(characters)) if characters.is_empty() => view! { <option value="0">"No characters online"</option> }.into_any(),
                                Some(Ok(characters)) => view! { {characters.into_iter().map(|character| view! { <option value=character.guid>{character.name.clone()} " (L" {character.level} ")"</option> }).collect_view()} }.into_any(),
                                Some(Err(error)) => view! { <option value="0">{error.to_string()}</option> }.into_any(),
                            }
                        }}
                    </select>
                </label>
                <label class="block text-muted-foreground" for="gm-type">"Type"
                    <select class="mt-2 block w-full border border-input bg-input px-2.5 py-2 text-foreground" id="gm-type" on:change=move |event| chat_type.set(event_target_value(&event))>
                        <option value="Whisper">"Whisper"</option>
                        <option value="Say">"Say"</option>
                        <option value="Channel">"Channel"</option>
                    </select>
                </label>
                <label class="block text-muted-foreground" for="gm-target">"Target player"
                    <input class="mt-2 block w-full border border-input bg-input px-2.5 py-2 text-foreground" id="gm-target" placeholder="Player name (for Whisper)" on:input=move |event| target.set(event_target_value(&event)) />
                </label>
                <label class="block text-muted-foreground" for="gm-channel">"Channel"
                    <input class="mt-2 block w-full border border-input bg-input px-2.5 py-2 text-foreground" id="gm-channel" placeholder="Channel name (for Channel)" on:input=move |event| channel.set(event_target_value(&event)) />
                </label>
            </div>
            <div class="mt-3 flex flex-col gap-3 sm:flex-row sm:items-end">
                <label class="block flex-1 text-muted-foreground" for="gm-message">"Message"
                    <input class="mt-2 block w-full border border-input bg-input px-2.5 py-2 text-foreground" id="gm-message" maxlength="512" placeholder="Message..." on:input=move |event| message.set(event_target_value(&event)) />
                </label>
                <Button button_type="button" on:click=do_send>"Send"</Button>
            </div>
            {move || send_error.get().map(|error| view! { <p class="mt-2 text-destructive">{error}</p> })}
            {move || {
                match send_result.get() {
                    Some(Ok(result)) => view! { <p class="mt-2 text-muted-foreground">{result.note}</p> }.into_any(),
                    Some(Err(error)) => view! { <p class="mt-2 text-destructive">{error.to_string()}</p> }.into_any(),
                    None => ().into_any(),
                }
            }}
        </section>
    }
}

/// Live feed backed by `chat-live.js` polling `/api/admin/chat/live`.
#[component]
fn LiveFeed() -> impl IntoView {
    view! {
        <section class="border border-border bg-card px-4 py-4 text-xs">
            <p class="font-medium text-foreground">"Live feed"</p>
            <p class="mt-2 text-muted-foreground">"Every message logged by the world server, refreshed every 1.5 seconds. Filter by type, channel, or a single player."</p>
            <div class="mt-4 flex flex-col gap-3 lg:flex-row lg:items-center">
                <label class="text-muted-foreground" for="chat-live-type">"Type"
                    <select class="mt-1 block border border-input bg-input px-2.5 py-2 text-foreground" id="chat-live-type">
                        <option value="">"All"</option>
                        <option value="Say">"Say"</option>
                        <option value="Yell">"Yell"</option>
                        <option value="Whisper">"Whisper"</option>
                        <option value="Emote">"Emote"</option>
                        <option value="Party">"Party"</option>
                        <option value="Raid">"Raid"</option>
                        <option value="Guild">"Guild"</option>
                        <option value="Officer">"Officer"</option>
                        <option value="Channel">"Channel"</option>
                    </select>
                </label>
                <label class="text-muted-foreground" for="chat-live-channel">"Channel"
                    <input class="mt-1 block border border-input bg-input px-2.5 py-2 text-foreground" id="chat-live-channel" placeholder="e.g. General, group:1, guild name..." />
                </label>
                <label class="text-muted-foreground" for="chat-live-player">"Player"
                    <input class="mt-1 block border border-input bg-input px-2.5 py-2 text-foreground" id="chat-live-player" placeholder="watch one player" />
                </label>
                <button class="border border-input px-3 py-2 text-muted-foreground hover:bg-input hover:text-foreground" id="chat-live-pause">"Pause"</button>
            </div>
            <div class="mt-4 max-h-[60vh] overflow-y-auto border border-border bg-background p-3 font-mono leading-relaxed" id="chat-live-feed"></div>
        </section>
        <Script defer="defer" src="/chat-live.js" />
    }
}

/// Aggregate volume per conversation, with drill-in to each conversation's history.
#[component]
fn ChannelsBrowser() -> impl IntoView {
    let overview = Resource::new(|| (), |_| portal::get_chat_overview());
    let selected = RwSignal::new(None::<(String, Option<String>)>);
    let messages = Resource::new(
        move || selected.get(),
        |selected| async move {
            match selected {
                Some((channel_type, channel_name)) => {
                    portal::get_chat_channel(channel_type, channel_name, 0, 200).await
                }
                None => Ok(Vec::new()),
            }
        },
    );

    view! {
        <div class="grid gap-4 xl:grid-cols-3">
            <section class="border border-border bg-card px-4 py-4 text-xs">
                <p class="font-medium text-foreground">"Conversations"</p>
                <Suspense fallback=move || view! { <p class="mt-3 text-muted-foreground">"Loading conversations..."</p> }>
                    {move || overview.get().map(move |result| render_channel_list(result, selected))}
                </Suspense>
            </section>
            <section class="border border-border bg-card px-4 py-4 text-xs xl:col-span-2">
                {move || selected.get().map(|(channel_type, channel_name)| {
                    let label = match &channel_name {
                        Some(name) => format!("{channel_type} · {name}"),
                        None => channel_type.clone(),
                    };
                    view! { <p class="font-medium text-foreground">{label}</p> }
                })}
                <Suspense fallback=move || view! { <p class="mt-3 text-muted-foreground">"Loading messages..."</p> }>
                    {move || messages.get().map(render_messages)}
                </Suspense>
            </section>
        </div>
    }
}

fn render_channel_list(
    result: Result<portal::ChatOverview, ServerFnError>,
    selected: RwSignal<Option<(String, Option<String>)>>,
) -> AnyView {
    match result {
        Ok(overview) if overview.channels.is_empty() => {
            view! { <p class="mt-3 text-muted-foreground">"No chat logged yet."</p> }.into_any()
        }
        Ok(overview) => {
            let total = overview.total_messages;
            view! {
                <p class="mt-3 text-muted-foreground">{format!("{total} messages logged")}</p>
                <ul class="mt-3 max-h-[55vh] divide-y divide-border overflow-y-auto">
                    {overview.channels.into_iter().map(move |channel| {
                        let channel_type = channel.channel_type.clone();
                        let channel_name = channel.channel_name.clone();
                        let label = match &channel_name {
                            Some(name) => format!("{channel_type} · {name}"),
                            None => channel_type.clone(),
                        };
                        let is_active = move || {
                            selected.get() == Some((channel.channel_type.clone(), channel.channel_name.clone()))
                        };
                        let class = move || {
                            if is_active() {
                                "flex w-full items-center justify-between gap-2 px-3 py-2 bg-primary/10 text-primary"
                            } else {
                                "flex w-full items-center justify-between gap-2 px-3 py-2 hover:bg-input"
                            }
                        };
                        view! {
                            <li>
                                <button class=class on:click=move |_| selected.set(Some((channel_type.clone(), channel_name.clone())))>
                                    <span>{label}</span>
                                    <span class="text-muted-foreground">{channel.message_count} " msgs"</span>
                                </button>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            }.into_any()
        }
        Err(error) => view! { <p class="mt-3 text-destructive">{error.to_string()}</p> }.into_any(),
    }
}

fn render_messages(result: Result<Vec<portal::ChatMessage>, ServerFnError>) -> AnyView {
    match result {
        Ok(messages) if messages.is_empty() => {
            view! { <p class="mt-3 text-muted-foreground">"No messages."</p> }.into_any()
        }
        Ok(messages) => render_message_list(messages),
        Err(error) => view! { <p class="mt-3 text-destructive">{error.to_string()}</p> }.into_any(),
    }
}

/// Player search and per-player / per-account history dive-in.
#[component]
fn PlayersBrowser() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let participants = Resource::new(
        move || search.get(),
        |search| portal::get_chat_participants(Some(search)),
    );
    let selected_player = RwSignal::new(None::<String>);
    let player_detail = Resource::new(
        move || selected_player.get(),
        |name| async move {
            match name {
                Some(name) => portal::get_player_chat(name).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let selected_account = RwSignal::new(None::<u32>);
    let account_chat = Resource::new(
        move || selected_account.get(),
        |account_id| async move {
            match account_id {
                Some(account_id) => portal::get_account_chat(account_id).await,
                None => Ok(Vec::new()),
            }
        },
    );

    view! {
        <div class="grid gap-4 xl:grid-cols-3">
            <section class="border border-border bg-card px-4 py-4 text-xs">
                <p class="font-medium text-foreground">"Player lookup"</p>
                <input class="mt-3 block w-full border border-input bg-input px-2.5 py-2 text-foreground" type="search" placeholder="Player name" on:input=move |event| search.set(event_target_value(&event)) />
                <p class="mt-2 text-muted-foreground">"Anyone who has spoken. Search to filter."</p>
                <Suspense fallback=move || view! { <p class="mt-3 text-muted-foreground">"Loading players..."</p> }>
                    {move || participants.get().map(move |result| render_participants(result, selected_player))}
                </Suspense>
            </section>
            <section class="border border-border bg-card px-4 py-4 text-xs xl:col-span-2">
                <Suspense fallback=move || view! { <p class="mt-3 text-muted-foreground">"Select a player on the left."</p> }>
                    {move || player_detail.get().map(move |result| render_player_detail(result, selected_account))}
                </Suspense>
                {move || selected_account.get().map(move |_| view! {
                    <div class="mt-6 border-t border-border pt-4">
                        <p class="font-medium text-foreground">"All activity for this account"</p>
                        <Suspense fallback=move || view! { <p class="mt-3 text-muted-foreground">"Loading account activity..."</p> }>
                            {move || account_chat.get().map(render_messages)}
                        </Suspense>
                    </div>
                })}
            </section>
        </div>
    }
}

fn render_participants(
    result: Result<Vec<portal::ChatParticipant>, ServerFnError>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    match result {
        Ok(participants) if participants.is_empty() => {
            view! { <p class="mt-3 text-muted-foreground">"No matching players."</p> }.into_any()
        }
        Ok(participants) => {
            view! {
                <ul class="mt-3 max-h-[55vh] divide-y divide-border overflow-y-auto">
                    {participants.into_iter().map(move |player| {
                        let name = player.name.clone();
                        let is_active_name = name.clone();
                        let is_active = move || selected.get().as_deref() == Some(is_active_name.as_str());
                        let class = move || {
                            if is_active() {
                                "flex w-full items-center justify-between gap-2 px-3 py-2 bg-primary/10 text-primary"
                            } else {
                                "flex w-full items-center justify-between gap-2 px-3 py-2 hover:bg-input"
                            }
                        };
                        view! {
                            <li>
                                <button class=class on:click=move |_| selected.set(Some(name.clone()))>
                                    <span>{player.name.clone()}</span>
                                    <span class="text-muted-foreground">{player.message_count} " msgs"</span>
                                </button>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            }.into_any()
        }
        Err(error) => {
            view! { <p class="mt-3 text-destructive">{error.to_string()}</p> }.into_any()
        }
    }
}

fn render_player_detail(
    result: Result<Option<portal::ChatPlayerDetail>, ServerFnError>,
    selected_account: RwSignal<Option<u32>>,
) -> AnyView {
    match result {
        Ok(None) => {
            view! { <p class="mt-3 text-muted-foreground">"Select a player on the left."</p> }
                .into_any()
        }
        Ok(Some(detail)) => {
            let name = detail.name.clone();
            let channels = detail.channels;
            let account_id = detail.account_id;
            let channel_count = channels.len();
            let message_count = detail.messages.len();
            view! {
                <p class="font-medium text-foreground">{format!("{name} — {message_count} messages across {channel_count} conversations")}</p>
                <p class="mt-2 text-muted-foreground">{channels.join(" · ")}</p>
                {account_id.map(move |account_id| view! {
                    <button class="mt-3 border border-input px-3 py-2 text-muted-foreground hover:bg-input hover:text-foreground" on:click=move |_| selected_account.set(Some(account_id))>
                        {format!("View all of account #{account_id}'s activity")}
                    </button>
                })}
                <div class="mt-4 max-h-[55vh] overflow-y-auto border border-border bg-background p-3 font-mono leading-relaxed">
                    {render_message_list(detail.messages)}
                </div>
            }.into_any()
        }
        Err(error) => view! { <p class="mt-3 text-destructive">{error.to_string()}</p> }.into_any(),
    }
}

fn render_message_list(messages: Vec<portal::ChatMessage>) -> AnyView {
    messages
        .into_iter()
        .map(|message| {
            let who = match &message.target_name {
                Some(target) => format!(
                    "{} -> {}",
                    message.sender_name.as_deref().unwrap_or("?"),
                    target
                ),
                None => message.sender_name.as_deref().unwrap_or("?").to_string(),
            };
            let channel = message
                .channel_name
                .as_deref()
                .map(|name| format!("[{name}] "))
                .unwrap_or_default();
            let color = match message.channel_type.as_str() {
                "Whisper" => "text-primary",
                "Guild" => "text-amber-400",
                "Officer" => "text-orange-400",
                "Party" => "text-emerald-400",
                "Raid" | "RaidLeader" | "RaidWarning" => "text-red-400",
                "Channel" => "text-sky-400",
                "Yell" => "text-orange-300",
                "Emote" => "text-fuchsia-400",
                _ => "text-muted-foreground",
            };
            view! {
                <div class="border-b border-border/40 py-1 last:border-b-0">
                    <span class="text-muted-foreground">{message.time}</span>
                    <span class=color>{" "}{message.channel_type.clone()}</span>
                    <span class="text-muted-foreground">{" "}{channel}</span>
                    <span class="text-foreground">{" "}{who}</span>
                    <span class="text-muted-foreground">": "</span>
                    <span class="text-foreground">{message.message.clone()}</span>
                </div>
            }
        })
        .collect_view()
        .into_any()
}
