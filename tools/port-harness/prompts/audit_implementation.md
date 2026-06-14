Compare documented C++ behaviour against the **existing** Rust implementation in the rcore repo.

## C++ Symbol
- Name: {symbol}
- File: {file}
- Lines: {startLine}-{endLine}

## C++ Source
```cpp
{cppSnippet}
```

## Behaviour Claims (from extract)
{claimsSummary}

## Flow Context
{flowContext}

## Rust Code Found in Repo
{rustCode}

Search notes: {rustSearchNotes}

## Instructions
The Rust code above was collected from `src/` (not a harness draft). Determine whether this C++ behaviour is already implemented.

Report:
- `implementation_status`: complete | partial | missing | incorrect
- `rust_locations`: where the behaviour lives in Rust (file paths relative to repo root)
- `coverage`: how many claims/branches appear covered
- `passed`: true only if implementation_status is complete and behaviour matches
- `missing_behaviours`: concise C++ behaviours that are not covered by Rust yet
- `planning_notes`: concrete implementation planning notes for the next Rust plan, including target module hints, data/state needed, branch ordering, validation rules, packet/API effects, and tests/fixtures to add
- `issues`: gaps — missing branches, wrong defaults, ordering differences, missing error paths

If no Rust code was found, status should be `missing` and passed false. In that case, `missing_behaviours` and `planning_notes` must still be populated from the C++ claims and flow context so the next plan-rust step has something actionable.
Do NOT assume behaviour from names alone — compare logic to claims.

Respond with JSON matching the schema.
