import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export type ProviderName = "openai" | "anthropic" | "openai_compat" | "cursor";

export interface FlowMapping {
  flow: string;
  rustTarget: string;
}

export interface HarnessConfig {
  referenceRoot: string;
  rustRoot: string;
  database: string;
  provider: {
    name: ProviderName;
    model: string;
    apiKeyEnv: string;
  };
  index: {
    maxChunkLines: number;
    excludePatterns: string[];
  };
  web: {
    host: string;
    port: number;
    vitePort: number;
  };
  jobs: {
    maxBatchSize: number;
    concurrency: number;
    backgroundConcurrency: number;
  };
  /** Optional symbol→flow pre-mappings applied after index when set */
  flowMappings?: Record<string, FlowMapping>;
  /** Optional extra context for assemble-flows prompts (domain-specific) */
  flowCategoriesHint?: string;
}

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function loadDotEnv(): void {
  const envPath = join(PACKAGE_ROOT, ".env");
  if (!existsSync(envPath)) return;
  for (const line of readFileSync(envPath, "utf-8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq <= 0) continue;
    const key = trimmed.slice(0, eq).trim();
    const val = trimmed.slice(eq + 1).trim().replace(/^["']|["']$/g, "");
    if (!process.env[key]) process.env[key] = val;
  }
}

export function getPackageRoot(): string {
  return PACKAGE_ROOT;
}

let configCache: HarnessConfig | null = null;

function defaultConfig(): HarnessConfig {
  return {
    referenceRoot: resolve(PACKAGE_ROOT, "../../reference/core"),
    rustRoot: resolve(PACKAGE_ROOT, "../.."),
    database: join(PACKAGE_ROOT, "port_harness.db"),
    provider: {
      name: "cursor",
      model: "composer-2.5",
      apiKeyEnv: "CURSOR_API_KEY",
    },
    index: {
      maxChunkLines: 150,
      excludePatterns: [],
    },
    web: {
      host: "127.0.0.1",
      port: 8787,
      vitePort: 5173,
    },
    jobs: {
      maxBatchSize: 10,
      concurrency: 2,
      backgroundConcurrency: 2,
    },
  };
}

function mergeConfig(defaults: HarnessConfig, user: Partial<HarnessConfig>): HarnessConfig {
  return {
    ...defaults,
    ...user,
    provider: { ...defaults.provider, ...user.provider },
    index: { ...defaults.index, ...user.index },
    web: { ...defaults.web, ...user.web },
    jobs: { ...defaults.jobs, ...user.jobs },
    flowMappings: user.flowMappings ?? defaults.flowMappings,
    flowCategoriesHint: user.flowCategoriesHint ?? defaults.flowCategoriesHint,
  };
}

export async function loadConfig(): Promise<HarnessConfig> {
  if (configCache) return configCache;
  loadDotEnv();

  const defaults = defaultConfig();
  const configPath = join(PACKAGE_ROOT, "port-harness.config.ts");
  if (existsSync(configPath)) {
    try {
      const mod = await import(pathToFileURL(configPath).href);
      configCache = mergeConfig(defaults, mod.default ?? {});
    } catch (err) {
      console.warn("Failed to load port-harness.config.ts:", err);
      configCache = defaults;
    }
  } else {
    configCache = defaults;
  }

  return configCache;
}

export function resolveReferencePath(config: HarnessConfig, ...parts: string[]): string {
  return resolve(config.referenceRoot, ...parts);
}

export function resolveRustPath(config: HarnessConfig, ...parts: string[]): string {
  return resolve(config.rustRoot, ...parts);
}

export function readPrompt(name: string): string {
  const path = join(PACKAGE_ROOT, "prompts", `${name}.md`);
  if (!existsSync(path)) {
    throw new Error(`Prompt not found: ${path}`);
  }
  return readFileSync(path, "utf-8");
}

export function fillPrompt(template: string, vars: Record<string, string>): string {
  let result = template;
  for (const [key, value] of Object.entries(vars)) {
    result = result.replaceAll(`{${key}}`, value);
  }
  return result;
}
