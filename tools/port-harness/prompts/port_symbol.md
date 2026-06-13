Port the following documented C++ symbol to Rust.

## Symbol
- Name: {symbol}
- File: {file}
- Lines: {startLine}-{endLine}

## Source Code
```cpp
{sourceSnippet}
```

## Behaviour Claims (acceptance criteria)
{claimsSummary}

## Target
- File: {targetRustFile}
- Symbol: {rustSymbolName}

## Instructions
- Port ONLY this symbol
- Preserve behaviour exactly per documented claims
- Do NOT refactor unless required by Rust
- Write idiomatic Rust: normal doc comments for non-obvious behaviour only
- Do NOT add `// PORT:` line-mapping comments in the Rust code
- Do NOT add `/// C++ SymbolName` or other C++ trace doc comments
- Add `// TODO:` comments only for unresolved semantics or missing dependencies
- Match conventions in the existing Rust module

Record source line mapping in `port_comments` metadata only — not as inline comments in `rust_code`.

Respond with the Rust code and any unresolved TODOs.
