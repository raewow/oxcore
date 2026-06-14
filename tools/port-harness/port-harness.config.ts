export default {
  referenceRoot: "../../reference/core",
  rustRoot: "../..",
  database: "./port_harness.db",
  provider: {
    name: "codex" as const,
    model: "gpt-5.4-mini",
    codexBin: "C:\\Users\\krist\\AppData\\Local\\OpenAI\\Codex\\bin\\codex.exe",
    codexPortSandbox: "workspace-write" as const,
    // To use Cursor instead:
    // name: "cursor" as const,
    // model: "composer-2.5",
    // apiKeyEnv: "CURSOR_API_KEY",
    // To use opencode instead:
    // name: "opencode" as const,
    // model: "anthropic/claude-sonnet-4-6",
    // apiKeyEnv is optional for opencode (auth is managed by the CLI)
  },
  index: {
    maxChunkLines: 150,
    // Per-method exclude patterns when indexing (glob suffix with *)
    excludePatterns: [] as string[],
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
  // Optional: pre-seed symbol→flow mappings after index (domain-specific).
  // See src/domains/spells.example.ts for a full spell-system example.
  // flowMappings: { "Unit::Update": { flow: "entity_tick", rustTarget: "src/world/entity.rs" } },
  // flowCategoriesHint: "## Known flows\n- entity_tick: Update, ...",
};
