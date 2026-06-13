Analyze the following C++ symbol and extract its behaviour documentation.

## Symbol
- Name: {symbol}
- File: {file}
- Lines: {startLine}-{endLine}

## Source Code
```cpp
{sourceSnippet}
```

## Instructions
For each claim you make, you MUST include:
- `file`: source file path
- `start_line` and `end_line`: exact line numbers from the source above
- `category`: one of input, output, branch, side_effect, assumption, danger, unknown

Do NOT invent behaviour. If uncertain, use category "unknown" with low confidence.
Flag dangerous C++ semantics (raw pointers, undefined order, macro expansion).

Respond with JSON matching the schema.
