# Upstream attribution

This file lists ideas taken from other repositories, so each one has a recorded source and license. Code from MIT and Apache-2.0 projects may be ported only with that project's notice; ideas from projects with noncommercial licenses are reimplemented, never copied.

| Codync piece | Repository | Commit | Source path | License | What was taken |
| --- | --- | --- | --- | --- | --- |
| `host/src/egress.rs` ([egress policy](egress-policy.md)) | [elie222/rakazo](https://github.com/elie222/rakazo) | `8d182d10734135615f0d8ea7e925a82ef756d647` | `packages/core/src/egress.ts`, `docs/bot-secrets.md` | Apache-2.0 | The policy only: HTTPS-only access, refusing metadata and link-local addresses, no redirects to another host, size limits. No code was copied. |
