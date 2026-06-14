Group the following indexed symbols into business flows for a C++ to Rust port.

## Source File
{file}

## Symbols and Claims
{symbolsSummary}

{flowCategoriesHint}

## Instructions
Infer logical business flows from symbol names, call relationships, and behaviour claims.
Group related methods (lifecycle, validation, state mutation, IO, networking, etc.).
Name flows descriptively in snake_case.

Keep each flow coherent and reviewable:
- One flow should describe one user-visible behaviour cluster, not an entire file unless the file is genuinely single-purpose.
- Prefer stable flow names that will survive incremental updates.
- Use the exact entry symbols that start the flow; avoid inventing new symbol names.
- Branches and mutations should be tied to line-cited evidence from the source file.
- If a symbol belongs to more than one plausible flow, choose the best home and mention the ambiguity in the description.
- Use `notes` to capture blockers, missing dependencies, or anything that prevents the flow from being finished yet.

For each flow, identify entry symbols, branches with line citations, and state mutations.
Respond with JSON matching this shape:

```json
{
  "flows": [
    {
      "name": "snake_case_flow_name",
      "description": "What the flow covers and why it matters",
      "notes": "Optional blockers, dependencies, or follow-up work",
      "entry_symbols": ["Qualified::Symbol"],
      "expected_behaviour": "Short behavioural summary",
      "risk_level": "low|medium|high|critical",
      "branches": [
        {
          "condition": "Branch condition",
          "behaviour": "Observed outcome",
          "file": "path/to/file.cpp",
          "start_line": 10,
          "end_line": 20
        }
      ],
      "mutations": [
        {
          "variable_or_field": "State being changed",
          "mutation_description": "How it changes",
          "file": "path/to/file.cpp",
          "start_line": 10,
          "end_line": 20
        }
      ]
    }
  ]
}
```

Once assembled, persist the result with the port-harness MCP tools:
- `save_flows` for bulk upserts
- `create_flow` for a new flow
- `update_flow` for iterative edits to an existing flow
