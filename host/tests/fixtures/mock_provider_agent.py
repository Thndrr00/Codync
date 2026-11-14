# Scripted ACP agent for the offline mock-provider test. It talks to `codync-host mock` (the base URL is argv[1])
# the way a model client would: a tool call, a provider error, and an approval asked in the chat.
#
#   tool:<text>      makes a tool call through the mock OpenAI chat endpoint, sends the tool result, and
#                    answers with the provider's final text
#   error:<text>     asks the mock provider to fail and reports the error it got
#   approval:<text>  asks permission to run the text, waits for the answer, and reports it
#   <anything else>  answers with the text
import json, sys, threading, urllib.error, urllib.request

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:1"
sid = "s1"
pending = {}
next_id = [100]
send_lock = threading.Lock()
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

def send(message):
    with send_lock:
        sys.stdout.write(json.dumps(message) + "\n")
        sys.stdout.flush()

def post(path, body):
    request = urllib.request.Request(
        BASE + path,
        data=json.dumps(body).encode("utf-8"),
        headers={"content-type": "application/json", "authorization": "Bearer offline-test"},
        method="POST",
    )
    try:
        with opener.open(request, timeout=10) as response:
            return response.status, json.loads(response.read().decode("utf-8") or "{}")
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read().decode("utf-8") or "{}")

def chat(messages, tools=None, stream=False):
    body = {"model": "mock-model", "messages": messages}
    if tools:
        body["tools"] = tools
    if stream:
        body["stream"] = True
    return post("/v1/chat/completions", body)

def reply(rid, text):
    send({"jsonrpc": "2.0", "method": "session/update", "params": {"sessionId": sid, "update": {
        "sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": text}}}})
    send({"jsonrpc": "2.0", "id": rid, "result": {"stopReason": "end_turn"}})

def tool_turn():
    tools = [{"type": "function", "function": {"name": "echo", "parameters": {"type": "object"}}}]
    messages = [{"role": "user", "content": "look this up [mock:tool]"}]
    status, body = chat(messages, tools=tools)
    if status != 200:
        return "tool call failed: %s %s" % (status, body)
    answer = body["choices"][0]["message"]
    call = answer["tool_calls"][0]
    messages.append(answer)
    messages.append({"role": "tool", "tool_call_id": call["id"], "content": "codync"})
    status, body = chat(messages, tools=tools)
    if status != 200:
        return "tool result failed: %s %s" % (status, body)
    return "tool: " + body["choices"][0]["message"]["content"]

def error_turn():
    status, body = chat([{"role": "user", "content": "please fail [mock:error]"}])
    return "provider error: %s %s" % (status, body.get("error", {}).get("message", "unknown"))

def approval_turn(rid, text):
    next_id[0] += 1
    pid = next_id[0]
    done = threading.Event()
    pending[pid] = done
    send({"jsonrpc": "2.0", "id": pid, "method": "session/request_permission", "params": {
        "sessionId": sid,
        "toolCall": {"toolCallId": "t1", "title": "Run " + text, "kind": "execute", "rawInput": {"command": text}},
        "options": [{"optionId": "yes", "name": "Allow", "kind": "allow_once"},
                    {"optionId": "no", "name": "Reject", "kind": "reject_once"}]}})
    done.wait()
    result = getattr(done, "answer", {})
    option = result.get("outcome", {}).get("optionId", "none")
    reply(rid, "approval outcome: " + str(option))

def turn(rid, text):
    if text.startswith("tool:"):
        reply(rid, tool_turn())
    elif text.startswith("error:"):
        reply(rid, error_turn())
    elif text.startswith("approval:"):
        approval_turn(rid, text[len("approval:"):].strip())
    else:
        reply(rid, "mock agent: " + text)

for line in sys.stdin:
    message = json.loads(line)
    if "method" not in message:
        done = pending.pop(message.get("id"), None)
        if done is not None:
            done.answer = message.get("result", {})
            done.set()
        continue
    method = message["method"]
    if method == "initialize":
        send({"jsonrpc": "2.0", "id": message["id"], "result": {
            "protocolVersion": 1, "agentCapabilities": {"loadSession": False}}})
    elif method == "session/new":
        send({"jsonrpc": "2.0", "id": message["id"], "result": {"sessionId": sid}})
    elif method == "session/prompt":
        text = " ".join(part.get("text", "") for part in message["params"]["prompt"])
        threading.Thread(target=turn, args=(message["id"], text.strip())).start()
