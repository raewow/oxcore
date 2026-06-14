import type Database from "better-sqlite3";
import type { HarnessConfig, ProviderName } from "../config.js";
import { getProviderFromConfig, type AgentProvider } from "../agents/provider.js";
import { getActiveProviderOverride } from "../db/repositories/settings.js";

const API_KEY_ENV: Partial<Record<ProviderName, string>> = {
  openai: "OPENAI_API_KEY",
  anthropic: "ANTHROPIC_API_KEY",
  openai_compat: "OPENAI_API_KEY",
  cursor: "CURSOR_API_KEY",
};

export async function resolveActiveProvider(
  db: Database.Database,
  config: HarnessConfig,
  onActivity?: (message: string) => void,
): Promise<AgentProvider> {
  const override = getActiveProviderOverride(db);
  const name = (override.name ?? config.provider.name) as ProviderName;
  // If provider is overridden but model isn't explicitly set, don't inherit
  // the old provider's model — let the new provider pick its own default.
  const model = override.model ?? (override.name ? undefined : config.provider.model);

  // Use override provider's expected API key env; keep config values for codex-specific opts
  const apiKeyEnv = API_KEY_ENV[name] ?? config.provider.apiKeyEnv;

  return getProviderFromConfig({
    name,
    model,
    apiKeyEnv,
    rustRoot: config.rustRoot,
    onActivity,
    codexBin: config.provider.codexBin,
    codexPortSandbox: config.provider.codexPortSandbox,
  });
}
