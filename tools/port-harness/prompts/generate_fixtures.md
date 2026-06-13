Generate test fixtures for the following business flow.

## Flow
- Name: {flowName}
- Description: {flowDescription}
- Expected behaviour: {expectedBehaviour}

## Branches
{branchesSummary}

## Instructions
Create JSON fixtures that exercise each branch. Each fixture needs:
- name: descriptive snake_case name
- description: what scenario this tests
- input: relevant state, parameters, and preconditions for the flow
- expected: expected return values, status codes, or observable side effects
- covers_branches: list of branch conditions exercised

Fixtures are for Rust unit tests, not C++ execution.
