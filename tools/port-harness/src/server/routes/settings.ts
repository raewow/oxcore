import { Hono } from "hono";
import type Database from "better-sqlite3";
import type { HarnessConfig, ProviderName } from "../../config.js";
import * as settingsRepo from "../../db/repositories/settings.js";

const AVAILABLE_PROVIDERS: ProviderName[] = [
  "codex",
  "opencode",
  "cursor",
  "openai",
  "anthropic",
  "openai_compat",
];

export function createSettingsRoutes(db: Database.Database, config: HarnessConfig): Hono {
  const app = new Hono();

  app.get("/", (c) => {
    const override = settingsRepo.getActiveProviderOverride(db);
    return c.json({
      active_provider: override.name ?? null,
      active_model: override.model ?? null,
      available_providers: AVAILABLE_PROVIDERS,
      config_default: {
        name: config.provider.name,
        model: config.provider.model,
      },
    });
  });

  app.put("/", async (c) => {
    const body = await c.req.json<{ active_provider?: string; active_model?: string }>();

    if (body.active_provider !== undefined) {
      if (body.active_provider === null || body.active_provider === "") {
        settingsRepo.deleteSetting(db, "active_provider");
      } else if (!AVAILABLE_PROVIDERS.includes(body.active_provider as ProviderName)) {
        return c.json({ error: `Unknown provider: ${body.active_provider}` }, 400);
      } else {
        settingsRepo.setSetting(db, "active_provider", body.active_provider);
      }
    }

    if (body.active_model !== undefined) {
      if (body.active_model === null || body.active_model === "") {
        settingsRepo.deleteSetting(db, "active_model");
      } else {
        settingsRepo.setSetting(db, "active_model", body.active_model);
      }
    }

    const override = settingsRepo.getActiveProviderOverride(db);
    return c.json({
      ok: true,
      active_provider: override.name ?? null,
      active_model: override.model ?? null,
    });
  });

  return app;
}
