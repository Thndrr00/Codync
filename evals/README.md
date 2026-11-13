# Bot evaluations

`tools/eval-bots.mjs` runs cases against a running host, through the same API the apps use. Start the
host (or use the Dev one), then:

```bash
node tools/eval-bots.mjs --home <CODYNC_HOME> --port 19223 evals/example.json
```

Compare two bots (or models behind them) on the same cases:

```bash
node tools/eval-bots.mjs --bots b1,b2 evals/example.json
```

Without files it reads every `evals/*.json`. A file is an array (or `{ "cases": [...] }`) of:

```json
[
  {
    "name": "finds where login lives",
    "botId": "b1",
    "prompt": "where is the login handled?",
    "expect": ["auth", "login"],
    "expectTools": ["search_workspace"],
    "timeoutSeconds": 90
  }
]
```

- `expect`: every string must appear in the final reply, case-insensitively.
- `expectTools`: every named tool must have been called during the turn.
- The script exits 1 when any case fails, so CI can run it against a seeded host.

Keep cases about observable behavior ("answers with a citation", "calls search_workspace"), not exact
wording, so a prompt tweak does not rewrite every case.
