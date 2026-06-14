Plan the Rust mapping for this documented C++ symbol.

## Symbol
- Name: {symbol}
- File: {file}
- Lines: {startLine}-{endLine}

## Behaviour Claims
{claimsSummary}

## Existing Rust Target
{existingRustPath}

## Latest Audit Context
{auditContext}

## Instructions
Decide:
- target_rust_file: where this should live in the Rust codebase
- rust_symbol_name: function/ method name in Rust
- structs/enums needed
- notes on Rust-specific adaptations required

Use the audit context to make the plan actionable. If the audit found missing or partial behaviour, the notes must explicitly cover those missing behaviours and any validation, state, packet/API, persistence, ordering, and fixture/test work needed.

Do NOT generate Rust code yet. Planning only.
