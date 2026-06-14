import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../api/client.js";

const MODEL_HINTS: Record<string, string> = {
  opencode: "anthropic/claude-sonnet-4-6",
  codex: "gpt-5.4-mini",
  cursor: "composer-2.5",
  openai: "gpt-4o",
  anthropic: "claude-sonnet-4-6",
  openai_compat: "your-model-name",
};

export default function Settings() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: ["settings"],
    queryFn: () => api.getSettings(),
  });

  const [provider, setProvider] = useState<string>("");
  const [model, setModel] = useState<string>("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (data) {
      setProvider(data.active_provider ?? "");
      setModel(data.active_model ?? "");
    }
  }, [data]);

  const mutation = useMutation({
    mutationFn: () =>
      api.updateSettings({
        active_provider: provider || null,
        active_model: model || null,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
      setSaved(true);
      setTimeout(() => setSaved(false), 2500);
    },
  });

  const effectiveProvider = provider || data?.config_default.name || "";
  const modelHint = MODEL_HINTS[effectiveProvider] ?? "";

  if (isLoading) return <p className="muted">Loading settings…</p>;

  return (
    <div className="page-content">
      <h1>Settings</h1>

      <section style={{ maxWidth: 520, marginTop: "1.5rem" }}>
        <h2 style={{ fontSize: "1rem", marginBottom: "1rem" }}>Active AI Agent</h2>

        <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
          <label style={{ display: "flex", flexDirection: "column", gap: "0.4rem" }}>
            <span style={{ fontSize: "0.85rem", color: "#94a3b8" }}>Provider</span>
            <select
              value={provider}
              onChange={(e) => {
                setProvider(e.target.value);
                setModel("");
              }}
              style={{
                background: "#1e293b",
                color: "#e2e8f0",
                border: "1px solid #334155",
                borderRadius: 4,
                padding: "0.4rem 0.6rem",
                fontSize: "0.9rem",
              }}
            >
              <option value="">— use config.ts default ({data?.config_default.name}) —</option>
              {data?.available_providers.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
          </label>

          <label style={{ display: "flex", flexDirection: "column", gap: "0.4rem" }}>
            <span style={{ fontSize: "0.85rem", color: "#94a3b8" }}>Model</span>
            <input
              type="text"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder={modelHint || data?.config_default.model || "model name"}
              style={{
                background: "#1e293b",
                color: "#e2e8f0",
                border: "1px solid #334155",
                borderRadius: 4,
                padding: "0.4rem 0.6rem",
                fontSize: "0.9rem",
                fontFamily: "monospace",
              }}
            />
            {!model && modelHint && (
              <span style={{ fontSize: "0.78rem", color: "#64748b" }}>
                Default for {effectiveProvider}: <code>{modelHint}</code>
              </span>
            )}
            {!provider && (
              <span style={{ fontSize: "0.78rem", color: "#64748b" }}>
                Config default: <code>{data?.config_default.model}</code>
              </span>
            )}
          </label>

          <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
            <button
              type="button"
              className="btn"
              onClick={() => mutation.mutate()}
              disabled={mutation.isPending}
            >
              {mutation.isPending ? "Saving…" : "Save"}
            </button>
            {saved && (
              <span style={{ fontSize: "0.85rem", color: "#4ade80" }}>Saved</span>
            )}
            {mutation.isError && (
              <span style={{ fontSize: "0.85rem", color: "#f87171" }}>
                {mutation.error instanceof Error ? mutation.error.message : "Error saving"}
              </span>
            )}
          </div>

          {(data?.active_provider || data?.active_model) && (
            <p style={{ fontSize: "0.8rem", color: "#64748b", marginTop: "0.25rem" }}>
              Active override:{" "}
              <code>
                {data.active_provider ?? data.config_default.name}
                {data.active_model ? ` / ${data.active_model}` : ""}
              </code>
              .{" "}
              <button
                type="button"
                style={{
                  background: "none",
                  border: "none",
                  color: "#94a3b8",
                  cursor: "pointer",
                  padding: 0,
                  fontSize: "0.8rem",
                  textDecoration: "underline",
                }}
                onClick={() => {
                  setProvider("");
                  setModel("");
                  api
                    .updateSettings({ active_provider: null, active_model: null })
                    .then(() => queryClient.invalidateQueries({ queryKey: ["settings"] }));
                }}
              >
                Reset to config default
              </button>
            </p>
          )}
        </div>

        <div
          style={{
            marginTop: "2rem",
            padding: "0.75rem 1rem",
            background: "#0f172a",
            borderRadius: 6,
            border: "1px solid #1e293b",
            fontSize: "0.82rem",
            color: "#64748b",
            lineHeight: 1.6,
          }}
        >
          <strong style={{ color: "#94a3b8" }}>Note:</strong> opencode uses CLI-managed
          subscription auth — run{" "}
          <code style={{ color: "#e2e8f0" }}>opencode auth login</code> once before using it.
          Changes take effect immediately for new jobs (no restart needed).
        </div>
      </section>
    </div>
  );
}
