"""Deterministic ACP agent for the credential-warning test: one MCP call whose arguments carry a
password, then a final reply. No provider or network access."""

import json
import sys
import threading

sid = "s1"


def send(value):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", **value}) + "\n")
    sys.stdout.flush()


def update(value):
    send({"method": "session/update", "params": {"sessionId": sid, "update": value}})


def turn(request):
    update(
        {
            "sessionUpdate": "tool_call",
            "toolCallId": "t0",
            "title": "Edit notes.md",
            "kind": "edit",
            "status": "completed",
            "rawInput": {"path": "notes.md", "token": "a-token-the-file-holds"},
            "content": [{"type": "diff", "path": "notes.md", "oldText": "a\n", "newText": "b\n"}],
        }
    )
    update(
        {
            "sessionUpdate": "tool_call",
            "toolCallId": "t1",
            "title": "mcp__gmail__send_message",
            "kind": "other",
            "status": "in_progress",
            "rawInput": {"to": "ana@example.com", "body": "Report", "password": "hunter2"},
        }
    )
    update(
        {
            "sessionUpdate": "tool_call_update",
            "toolCallId": "t1",
            "status": "completed",
            "content": [{"type": "text", "text": "sent"}],
        }
    )
    update({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": "Sent it."}})
    send({"id": request, "result": {"stopReason": "end_turn"}})


for line in sys.stdin:
    message = json.loads(line)
    if "method" not in message:
        continue
    method = message["method"]
    if method == "initialize":
        send({"id": message["id"], "result": {"protocolVersion": 1, "agentCapabilities": {"loadSession": False}}})
    elif method == "session/new":
        send({"id": message["id"], "result": {"sessionId": sid}})
    elif method == "session/prompt":
        threading.Thread(target=turn, args=(message["id"],), daemon=True).start()
