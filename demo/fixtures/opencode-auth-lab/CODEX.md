# CODEX

Use this file as a compatibility seed when a GPT/Codex-style model is active inside OpenCode.
CTX should ingest these rules into graph memory so the model can fetch only the directives needed for the current task.

## Implementation Rules

- Preserve strong assertions in refresh-token tests unless behavior intentionally changes.
- Fix the root cause in the route, session, or token helper instead of adding a shallow workaround.
- Keep documentation and benchmark-facing examples aligned if command behavior changes.

## Retrieval Habits

- Prefer auth fixtures when debugging token rotation failures.
- Start from the smallest useful context pack, then expand only if the first retrieval is insufficient.
- Treat graph memory as the primary source for repeated project habits.
