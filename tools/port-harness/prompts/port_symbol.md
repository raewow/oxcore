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
- Add `// PORT: {file}:L{startLine}-L{endLine}` comments
- Add TODO comments for unresolved C++ semantics
- Match conventions in the existing Rust module

Respond with the Rust code and any unresolved TODOs.
