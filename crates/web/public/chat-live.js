(() => {
  if (window.__oxcoreChatLivePoller) return;
  window.__oxcoreChatLivePoller = true;

  let since = 0;

  function feed() {
    return document.getElementById("chat-live-feed");
  }

  function showModeration(name) {
    document.getElementById("chat-live-moderation")?.remove();
    const popover = document.createElement("div");
    popover.id = "chat-live-moderation";
    popover.className = "fixed bottom-5 right-5 z-50 w-52 border border-primary/30 bg-card p-3 shadow-xl shadow-black/40";
    popover.innerHTML = `<div class="flex items-start justify-between gap-3"><div><p class="text-[10px] font-bold uppercase tracking-[0.15em] text-muted-foreground">Quick actions</p><p class="mt-1 text-sm font-semibold text-foreground"></p></div><button class="text-muted-foreground hover:text-foreground" type="button" aria-label="Close moderation actions">×</button></div><div class="mt-3 grid grid-cols-2 gap-1"></div><p class="mt-2 text-[10px] text-muted-foreground">Actions are not enabled yet.</p>`;
    popover.querySelector("p.text-sm").textContent = name;
    popover.querySelector("button").addEventListener("click", () => popover.remove());
    const actions = popover.querySelector(".grid");
    ["Mute", "Kick", "Ban", "View account"].forEach((label) => {
      const action = document.createElement("button");
      action.className = "border border-input px-2 py-2 text-left text-[11px] text-muted-foreground hover:border-primary/50 hover:bg-input hover:text-foreground";
      action.type = "button";
      action.textContent = label;
      actions.append(action);
    });
    document.body.append(popover);
  }

  function formatTime(unixSeconds) {
    const now = Math.floor(Date.now() / 1000);
    const seconds = Math.max(0, now - unixSeconds);
    if (seconds < 60) return "now";
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
    return `${Math.floor(seconds / 86400)}d`;
  }

  function messageLine(message) {
    const row = document.createElement("article");
    row.className = "flex gap-3 border-b border-border/50 py-3 last:border-b-0";

    const avatar = document.createElement("span");
    avatar.className = "grid h-8 w-8 shrink-0 place-items-center bg-input text-xs font-bold text-primary";
    avatar.textContent = (message.sender_name || "?").slice(0, 1);

    const content = document.createElement("div");
    content.className = "min-w-0 flex-1";
    const meta = document.createElement("div");
    meta.className = "flex flex-wrap items-baseline gap-x-2";
    const sender = document.createElement("button");
    sender.className = "font-semibold text-foreground hover:text-primary";
    sender.type = "button";
    sender.textContent = message.sender_name || "Unknown";
    sender.addEventListener("click", () => showModeration(sender.textContent));
    const time = document.createElement("span");
    time.className = "text-[10px] text-muted-foreground";
    time.textContent = formatTime(message.time);
    const type = document.createElement("span");
    type.className = "text-[10px] uppercase tracking-wide text-primary/80";
    type.textContent = message.channel_type;
    meta.append(sender, time, type);
    if (message.channel_name) {
      const channel = document.createElement("span");
      channel.className = "text-[10px] text-muted-foreground";
      channel.textContent = `# ${message.channel_name}`;
      meta.append(channel);
    }
    content.append(meta);
    if (message.target_name) {
      const target = document.createElement("p");
      target.className = "mt-0.5 text-[11px] text-muted-foreground";
      target.textContent = `to ${message.target_name}`;
      content.append(target);
    }
    const text = document.createElement("p");
    text.className = "mt-1 break-words text-sm leading-6 text-foreground";
    text.textContent = message.message;
    content.append(text);
    row.append(avatar, content);
    return row;
  }

  function render(messages) {
    const currentFeed = feed();
    if (!currentFeed) return;
    currentFeed.replaceChildren(...messages.slice().reverse().map(messageLine));
    currentFeed.scrollTop = currentFeed.scrollHeight;
  }

  async function refresh() {
    const currentFeed = feed();
    if (!currentFeed) return;
    try {
      const response = await fetch(`/api/admin/chat/live?since=${since}&limit=300`, { credentials: "same-origin" });
      if (!response.ok) return;
      const messages = await response.json();
      if (!Array.isArray(messages) || messages.length === 0) return;
      if (since > 0) {
        for (const message of messages) currentFeed.appendChild(messageLine(message));
      } else {
        render(messages);
      }
      since = messages[messages.length - 1].id;
      currentFeed.scrollTop = currentFeed.scrollHeight;
    } catch {
      // Transient network errors are retried by the next polling interval.
    }
  }

  refresh();
  window.setInterval(refresh, 1500);
})();
