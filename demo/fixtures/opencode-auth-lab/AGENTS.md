# AGENTS

This project simulates a real auth and session-management codebase used to validate CTX.
Read the repository carefully before changing token behavior.
The refresh-token path is sensitive because it affects session continuity, auditability, and test stability.
Do not make blind changes just because a test is failing.
Prefer understanding the auth route, session boundary, retry flow, and audit implications before editing code.
When debugging, keep the explanation readable for future maintainers and avoid shallow workaround reasoning.
If a failure appears in CI, preserve the root cause in the explanation and avoid replacing the failure with a weaker assertion.
If command behavior changes, keep documentation and examples consistent.
Avoid broad prompt stuffing when the same information can be retrieved precisely from local context.
This repository is intentionally small, but you should treat it like a production-adjacent service where auth regressions are costly.
The code is organized so that route logic, session logic, token helpers, retries, and audit trails can be inspected independently.
Changes that look local often have side effects across retry timing, token rotation semantics, and test expectations.
Prefer reading the route and session flow together before making assumptions about the source of a failure.
If a bug involves refresh token rotation, think through what should happen on the first request, the second request, and when the old token is replayed.
When in doubt, bias toward explicit state transitions and precise naming instead of clever shortcuts.

## Project Habits

This codebase favors disciplined debugging and small, reversible changes.
The intended workflow is: inspect the relevant code path, identify the real failure mode, write or run the narrowest useful test, make the change, and then validate the surrounding behavior.
Do not patch one layer without checking whether the behavior should instead live in the route, session manager, token utility, or retry helper.
If you touch command behavior or testing expectations, remember that this fixture is also used for documentation and benchmark demonstrations.

## Testing Expectations

Before finishing any change, verify the narrow auth-related tests first and then decide whether the surrounding behavior needs wider confirmation.
If an assertion is failing, prefer preserving the strong assertion and fixing the underlying state or data shape.
Do not silently weaken tests just to get green output.
If behavior changes intentionally, update the expected reasoning in the demo docs or benchmark material as needed.

## Auth And Token Rules

Refresh-token rotation must remain explainable.
If a token is rotated, the route and session layer should agree on what becomes current and what becomes stale.
Be careful with any logic that marks tokens as rotated, revoked, consumed, or invalidated.
Audit data should be specific enough to understand what happened without exposing secrets.
Avoid logging raw secrets or sensitive payloads in examples, tests, or debug output.

## OpenCode And CTX Workflow

This fixture exists to prove that CTX works best as an OpenCode-native workflow.
Prefer using precise retrieval, graph, and memory commands instead of rereading every file or restating the entire project guide.
If project habits are needed repeatedly, they should be discoverable as graph memory rather than treated as one giant markdown blob every time.
Use the smallest amount of context that still explains the bug or implementation task clearly.

## Documentation And Benchmarks

This repository is part of the public demonstration of CTX.
That means changes here can affect:
- the graph-memory bootstrap flow
- the demo walkthrough
- the benchmark token comparison
- the expected narrative around why graph memory beats a large markdown reread

If you intentionally change the project habits or expected answers, verify whether the benchmark reports and walkthrough still tell the truth.
The goal is not just to make tests pass, but to keep the fixture credible as a release-quality example.

- Run targeted auth tests before completion.
- Fix root cause instead of bypassing refresh-token failures.
- Update docs when command behavior changes.
- Prefer graph memory lookup over rereading the full project guide.
- Keep route, session, and token semantics aligned when editing auth behavior.
- Preserve strong assertions in tests unless the behavior itself is intentionally changing.
- Record audit-relevant behavior with precise, non-secret explanations.
- Use narrow retrieval and compact context before broad file dumping.
- If benchmark-facing behavior changes, re-check the demo walkthrough and reports.
