# MeshInspector UI Validation Evidence

This directory stores local validation evidence for the official MeshLib workbench UI backed by the FastAPI/Rust geometry stack.

Generated artifacts:

- `playwright-results.json`: structured result output from `bun run e2e:workbench`.
- `playwright-html/`: Playwright HTML report.
- `meshinspector-workbench-algorithms-*/workbench-command-matrix-results.json`: per-command matrix result attachment.
- `meshinspector-workbench-algorithms-*/*.png`: failure screenshots captured for individual commands.

Required local services:

```bash
cd meshinspector-backend
uv run uvicorn main:app --host 127.0.0.1 --port 48100

cd meshinspector-frontend
NEXT_PUBLIC_API_URL=http://127.0.0.1:48100 bun run dev -- --hostname 127.0.0.1 --port 48101
```

Run validation:

```bash
cd meshinspector-frontend
MESHINSPECTOR_API_URL=http://127.0.0.1:48100 MESHINSPECTOR_BASE_URL=http://127.0.0.1:48101 bun run e2e:workbench
```

Summarize saved official UI harness evidence:

```bash
node .codex/ui-validation/summarize-official-ui-coverage.mjs
```

Generated artifacts:

- `rest-command-coverage-latest.json`: machine-readable Rust-backed REST command coverage.
- `rest-command-coverage-latest.md`: compact coverage summary and evidence index.
- `direct-computer-use-blocker-2026-06-13.md`: direct Computer Use capture failure evidence.
