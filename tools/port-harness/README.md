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
