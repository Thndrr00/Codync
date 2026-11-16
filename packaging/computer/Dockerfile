# A bot's container computer (docs/features/computer-containers.md): a shell and the usual command-line tools
# in its own Docker container. Codync mounts the bot's workspace at /workspace and its browser profile at
# /profile, drops every capability and runs it with gVisor when the engine has it.
#
#   docker build -t codync/computer:1 packaging/computer
#
# The bot's container is recreated when its image label changes; its workspace and home volume stay.
FROM debian:bookworm-slim

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates curl git jq nodejs npm openssh-client python3 ripgrep \
    && rm -rf /var/lib/apt/lists/*

ENV HOME=/home/codync \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

RUN mkdir -p /home/codync /workspace /profile

WORKDIR /workspace

# Kept alive by the host; commands arrive through `docker exec`.
CMD ["sleep", "infinity"]
