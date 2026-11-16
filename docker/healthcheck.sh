#!/bin/sh
# Docker health check: the host's own GET /health, which answers the network without a token
# (host/src/api/http.rs). Port comes from CODYNC_PORT, the same value the entrypoint passes
# to `serve`.
set -eu

url="http://127.0.0.1:${CODYNC_PORT:-19222}/health"

if command -v curl >/dev/null 2>&1; then
    curl -fsS --max-time 4 "$url" >/dev/null
elif command -v wget >/dev/null 2>&1; then
    wget -q -T 4 -O /dev/null "$url"
else
    echo "codync-healthcheck: neither curl nor wget is installed" >&2
    exit 1
fi
