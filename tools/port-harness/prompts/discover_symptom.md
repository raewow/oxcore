Investigate a reported bug or missing behavior in a C++ game server being ported to Rust.

## User Report
{query}

## Port Harness DB Seed Hits
These symbols/claims/flows matched the query in the migration database (may be incomplete if files are not indexed):

```json
{seedHits}
```

## Call Graph Neighbors (1-hop)
```json
{callGraphNeighbors}
```

## File Port Status for Matched Files
```json
{fileStats}
```

## Repository Layout
- Reference C++ source: `{referenceRoot}` (relative to repo root: `reference/core`)
- Rust port target: `{rustRoot}` (repo `src/` tree)
- Port harness tracks indexed symbols in SQLite; unindexed `.cpp` files won't appear in seed hits.

## Instructions
1. Search the reference C++ codebase for code paths related to the reported symptom.
2. Cross-check the Rust `src/` tree for partial or complete ports of relevant logic.
3. Identify **all symbols and files** that likely need inspection or porting — include handlers, eligibility checks, state mutations, DB lookups, and packet handlers.
4. For each candidate already in the seed hits, set `in_index: true` and include `task_id`/`task_status` when available.
5. For reference files not yet indexed, list them in `unindexed_files`.
6. Rank candidates by relevance (`high` / `medium` / `low`) with clear reasoning.
7. Suggest concrete next steps (index file, extract behaviour, verify port, inspect source, check rust).

Respond with JSON matching the schema only.
