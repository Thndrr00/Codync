#!/bin/sh
# Starts the host on the port and address the container was given. Any other command runs as
# it is, so `docker run --rm codync-host:local info` answers like the installed binary:
#
#   docker compose exec codync codync-host pair
set -eu

if [ "$#" -eq 0 ] || [ "$1" = serve ]; then
    exec codync-host serve --bind "${CODYNC_BIND:-0.0.0.0}" --port "${CODYNC_PORT:-19222}"
fi

exec "$@"
