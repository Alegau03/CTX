# CLAUDE

Use this file as a compatibility seed when a Claude-family model is active inside OpenCode.
The real goal is still to import these rules into CTX graph memory and retrieve them only when relevant.

## Auth Priorities

- Keep route, session, and token semantics aligned when editing refresh-token behavior.
- Prefer root-cause fixes over weakening assertions or bypassing failing auth tests.
- Preserve audit visibility in explanations without exposing secrets or raw token material.

## Working Style

- Read the refresh route and session logic together before changing token rotation code.
- Prefer narrow retrieval and compact context over rereading the full project guide.
- Re-run the targeted auth tests before considering the task complete.
