(() => {
  const feed = document.getElementById("chat-live-feed");
  if (!feed) return;
  const typeSelect = document.getElementById("chat-live-type");
  const channelInput = document.getElementById("chat-live-channel");
  const playerInput = document.getElementById("chat-live-player");
  const pauseButton = document.getElementById("chat-live-pause");

  let paused = false;
  let since = 0;

  function currentQuery() {
    const params = new URLSearchParams();
    const type = typeSelect ? typeSelect.value : "";
    const channel = channelInput ? channelInput.value.trim() : "";
    const player = playerInput ? playerInput.value.trim() : "";
    if (type) params.set("channel_type", type);
    if (channel) params.set("channel_name", channel);
    if (player) params.set("player", player);
    params.set("limit", "300");
    return params;
  }

  function formatTime(unixSeconds) {
    const now = Math.floor(Date.now() / 1000);
    const secs = Math.max(0, now - unixSeconds);
    if (secs < 3600) return `${secs}s ago`;
    if (secs < 86400) return `${Math.floor(secs / 60)}m ago`;
    return `${Math.floor(secs / 86400)}d ago`;
  }

  function messageLine(message) {
    const el = document.createElement("div");
    el.className = "border-b border-black/20 py-1 last:border-b-0";
    const who = message.target_name
      ? `${message.sender_name || "?"} -> ${message.target_name}`
      : (message.sender_name || "?");
    const channel = message.channel_name ? `[${message.channel_name}] ` : "";
    el.innerHTML = [
      `<span class="text-muted-foreground">${formatTime(message.time)}</span>`,
      ` <span class="text-muted-foreground">${escapeHtml(message.channel_type)}</span>`,
      ` <span class="text-muted-foreground">${escapeHtml(channel)}</span>`,
      ` <span class="text-foreground">${escapeHtml(who)}</span>`,
      `<span class="text-muted-foreground">: </span>`,
      `<span class="text-foreground">${escapeHtml(message.message)}</span>`,
    ].join("");
    return el;
  }

  function escapeHtml(value) {
    return value.replace(/[&<>"']/g, (char) => ({
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#39;",
    })[char]);
  }

  function render(messages) {
    feed.replaceChildren(...messages.map(messageLine));
    feed.scrollTop = feed.scrollHeight;
  }

  async function refresh() {
    if (paused || !feed) return;
    const params = currentQuery();
    params.set("since", String(since));
    try {
      const response = await fetch(`/api/admin/chat/live?${params}`, { credentials: "same-origin" });
      if (!response.ok) return;
      const messages = await response.json();
      if (!Array.isArray(messages)) return;
      if (messages.length === 0) return;
      if (since > 0) {
        for (const message of messages) {
          feed.appendChild(messageLine(message));
        }
        feed.scrollTop = feed.scrollHeight;
        since = messages[messages.length - 1].id;
      } else {
        since = messages[0].id;
        render(messages);
      }
    } catch {
      // transient network errors are fine; retry on the next tick
    }
  }

  function reset() {
    since = 0;
    feed.replaceChildren();
    refresh();
  }

  if (typeSelect) typeSelect.addEventListener("change", reset);
  if (channelInput) channelInput.addEventListener("input", reset);
  if (playerInput) playerInput.addEventListener("input", reset);
  if (pauseButton) {
    pauseButton.addEventListener("click", () => {
      paused = !paused;
      pauseButton.textContent = paused ? "Resume" : "Pause";
    });
  }

  refresh();
  window.setInterval(refresh, 1500);
})();
