# OpenCode Auth Lab

This fixture repository exists to validate CTX inside OpenCode.

It is intentionally small, but realistic enough to stress the core product story:

- refresh-token debugging on a TypeScript codebase
- AGENTS-style project habits starting in markdown
- graph memory bootstrap and topic search
- retrieval, prune, pack, and MCP flows
- benchmark comparison between markdown memory and graph memory

The intended validation story is:

1. bootstrap CTX in the repo
2. install the OpenCode integration
3. import AGENTS-style files into graph memory
4. search only the rules relevant to auth and tests
5. prune noisy Vitest logs
6. build a compact pack for the fix task
7. compare graph memory against markdown memory with reproducible benchmark reports

## Log Demo Setup

Install the fixture dependencies before testing `/ctx-prune-logs` inside OpenCode:

```bash
npm install
```

Then use:

```text
/ctx-prune-logs npm run test:auth
```

Expected outcome:

- no `vitest: command not found`
- pruned auth failure output instead of raw shell noise
- readable root cause from the refresh-route assertion failure
