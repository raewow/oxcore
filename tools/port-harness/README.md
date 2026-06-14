# Port Harness

C++ to Rust migration agent harness for rcore. Works across the full reference C++ tree under `reference/core/`.

## Setup

```bash
cd tools/port-harness
npm install
export CURSOR_API_KEY=...   # from cursor.com/settings
```

Requires Node.js 20+, Cursor CLI/SDK, and reference C++ at `reference/core/`.

## Agent provider

**Default: Cursor SDK** (`Agent.prompt` against local repo). Set `CURSOR_API_KEY`.

Other providers (optional): `openai`, `anthropic`, `openai_compat` — override in `port-harness.config.ts` or `--provider`.

## CLI

```bash
npm run dev -- index src/game/Entities/Unit.cpp
npm run dev -- status --file src/game/Entities/Unit.cpp
npm run dev -- serve                    # API :8787
npm run dev:web                         # API + dashboard :5173
```

## Pipeline (LLM via Cursor)

```bash
npm run dev -- extract --symbol Unit::Update
npm run dev -- assemble-flows --file src/game/Entities/Unit.cpp
npm run dev -- fixtures --flow entity_tick          # JSON only → fixtures/
npm run dev -- fixtures --flow entity_tick --write-rust-tests  # optional stubs in generated/
npm run dev -- plan-rust --task 1
npm run dev -- audit-rust --flow 10          # audit all symbols in a flow
npm run dev -- audit-rust --task 42          # audit one symbol
npm run dev -- port --task 1 --write
npm run dev -- verify --task 1                     # doc review only
npm run dev -- verify --task 1 --cargo             # also run cargo test
```

## Feature discovery

Feature groups let you collect related files, flows, and tasks, then queue work from that set.

```bash
# Create a spells feature and attach known entry files.
npm run dev -- feature create spells \
  --description "Spell casting, targeting, effects, resources, and spell packets" \
  --dir src/game/Spells \
  --file src/game/Handlers/SpellHandler.cpp src/game/Server/Packets/Spell.cpp \
  --match Server/Protocol/Opcodes

# Run discovery, assign discovered candidates to the feature, and queue index/extract/verify jobs.
npm run dev -- feature discover spells \
  --query "Discover all C++ code needed to port spell casting, target selection, spell effects, cooldowns, power/reagents, channels, delayed spells, spell handlers, spell packets, and spell opcodes" \
  --sync --assign --queue

# Inspect the feature and queue additional work as needed.
npm run dev -- feature show spells
npm run dev -- feature queue spells --stage pipeline
npm run dev -- feature queue spells --stage extract
npm run dev -- feature queue spells --stage assemble-flows
```

Useful commands:

- `feature suggest <feature> --accept-all` refreshes keyword suggestions and accepts them.
- `feature create/assign --dir <path>` assigns every `.cpp` under a reference directory.
- `feature create/assign --match <text>` assigns files whose path contains a substring, useful for opcode tables.
- Add `--include-headers` with `--dir` or `--match` when header-only declarations matter.
- `feature queue <feature> --stage pipeline` queues index, extract, and flow assembly per assigned `.cpp`.
- `feature queue <feature> --stage index` indexes assigned `.cpp` files.
- `feature queue <feature> --stage extract --status discovered` documents assigned discovered tasks.

## Domain config (optional)

For known symbol→flow mappings or assemble-flow hints, add to `port-harness.config.ts`:

- `flowMappings` — applied after index (CLI: `--apply-mappings`, or automatically via the web UI when set)
- `flowCategoriesHint` — extra context for the assemble-flows LLM prompt
- `jobs.concurrency` — how many pipeline jobs run in parallel (default: 2)

See `src/domains/spells.example.ts` for a spell-system reference. Run `npm run dev -- pilot spells` to bootstrap that example only.

## What gets stored where

| Artifact | Location |
|----------|----------|
| Symbol index, tasks, claims | SQLite `port_harness.db` |
| Behaviour docs | `tools/port-harness/docs/` |
| Implementation audits | `tools/port-harness/docs/audits/` |
| Test fixture **ideas** (JSON) | `tools/port-harness/fixtures/` |
| Draft Rust port output | `tools/port-harness/docs/ports/` |

The harness does **not** write into `src/` or `tests/` until you explicitly port and land code yourself. Rust test stubs are opt-in under `generated/` for reference only.

## MCP server (Claude Code)

Exposes the DB (symbol index, call graph, behaviour claims, feature tracking) to Claude Code as tools, so you query/track porting without one-off scripts.

```bash
npm run mcp   # stdio MCP server
```

Registered for the repo via `../../.mcp.json`; restart Claude Code to load it, then drive it with the `/port-feature` skill. Tools:

- `find_symbol`, `symbol_callees`, `symbol_callers` — search + walk the call graph
- `behaviour_claims` — the extracted spec to preserve when porting
- `list_flows`, `flow_details` — inspect flows after document stage, including branches, mutations, and linked tasks
- `feature_coverage` — "is every detail mapped?" (closure %, symbols to pull in, top gaps)
- `feature_status`, `next_tasks` — progress + what to work on next, now with flow context on tasks
- `set_task_status` — the only mutating tool; advances a symbol along the porting ladder

## Web dashboard

http://127.0.0.1:5173

- **Files** — search reference C++ tree, Index / Document / Flows / All per file
- **Tasks** — symbol-level grid with bulk actions
- **Jobs** — pipeline job monitor (pause/resume, retry, continue)

Loads `tools/port-harness/.env` automatically (e.g. `CURSOR_API_KEY=...`).

## Tests

```bash
npm test   # harness unit tests (vitest)
```
