# Egress policy

`host/src/egress.rs` decides which destinations an outgoing request may reach when its URL comes from outside the host: a connector URL the user added or the MCP Registry listed, the endpoints named in a server's OAuth metadata, the download links in a GitHub API response, or an agent archive named by the ACP registry. Fixed first-party URLs (relay, cloud, updates, analytics, voice providers, Composio, nodejs.org, the ACP registry JSON itself) keep using the shared `crate::http()` client.

## Rules

`egress::client(policy, url)` resolves the host, checks every address it resolves to, and returns a client with those addresses pinned (`resolve_to_addrs`), so a second DNS lookup can't return a different address (DNS rebinding). A pinned client is reused for five minutes per `(policy, scheme, host, port)`.

| Address | `Public` / `Only` | `UserChosen` |
| --- | --- | --- |
| Public | HTTPS | HTTPS |
| Loopback, RFC 1918, CGNAT `100.64/10` (Tailscale), unique local and site-local IPv6 | refused | HTTPS or HTTP |
| Link-local `169.254/16` and `fe80::/10` (cloud metadata), `100.100.100.200`, `fd00:ec2::254`, unspecified, multicast, broadcast, documentation, reserved | refused | refused |

IPv6 addresses that reach an IPv4 address (IPv4-mapped `::ffff:a.b.c.d`, IPv4-compatible `::a.b.c.d`, NAT64 `64:ff9b::/96` and `64:ff9b:1::/48`, 6to4 `2002::/16`) are judged by that IPv4 address. Plain HTTP is allowed only when every resolved address is on the local network. A host that resolves to both public and local addresses is treated as local.

| Policy | Used for |
| --- | --- |
| `Connector::egress()`: `UserChosen` for a connector the user added (custom URL, pasted config), `Public` for one from the MCP Registry | The connector's own URL: the check when it is added, the install probe, and the `codync-host mcp remote` proxy (`connectorTarget` returns `public` so the proxy knows) |
| `egress::derived(connector policy, url)` | Anything a connector's server names: OAuth resource and authorization-server metadata, client registration, the token endpoint, and the older SSE transport's POST endpoint. It is `UserChosen` only for a user-added connector whose every address is on the local network, and `Public` otherwise, including when the lookup fails. This way a public server can't point the host at the user's LAN. |
| `Public` | MCP Registry JSON; agent archives from the ACP registry (`egress::get_following`); a bot's `api_request` calls ([bot secrets](../features/connector-credentials.md#bot-secrets)) |
| `Only(["api.github.com", "raw.githubusercontent.com"])` | Skill catalog listings and `SKILL.md` files, and skill downloads |

Redirects are followed only within the first request's origin (at most five), because only that host's addresses are pinned. A redirect to another host is returned as the 3xx response. `egress::get_following` is for downloads served from a CDN (GitHub release assets): it follows up to five redirects to other hosts, checking and pinning each hop under the same policy.

`egress::body(res, max)` and `egress::json(res, max)` read a response body and return an error once it exceeds `max` bytes, instead of truncating it: 8 MB for registry and GitHub JSON, 1 MB for skill files and OAuth metadata or token responses. MCP traffic is streamed and has no size limit; agent archives are verified by their SHA-256. Each call site keeps its existing timeouts.

Refusals are logged as `egress refused` with the host, the policy and the reason. The URL itself is not logged, since a query string can carry a token.

## Limits

- When a system proxy is configured (`HTTPS_PROXY`), reqwest sends requests through it, so the proxy resolves the name and the address check runs on the host's own lookup only.
- Adding a remote connector resolves its host, so it fails while the computer is offline.
- Local stdio connectors and agent harnesses make their own network calls; this policy covers only requests the host makes itself.

The idea comes from rakazo's `packages/core/src/egress.ts` (Apache-2.0). It was reimplemented, not copied; see [upstream attribution](upstream-attribution.md).
