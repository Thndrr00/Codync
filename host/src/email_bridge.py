"""Minimal IMAP/SMTP bridge for Codync email accounts.

Reads one JSON request from stdin and writes one JSON answer to stdout. Uses only the Python
standard library; no message is stored here. The password comes on stdin, never in argv or files.
"""

import email
import imaplib
import json
import smtplib
import ssl
import sys
from email.header import decode_header, make_header
from email.message import EmailMessage


def decoded(value):
    if not value:
        return ""
    try:
        return str(make_header(decode_header(value)))
    except Exception:
        return value


def connect_imap(account):
    host = account["imapHost"]
    port = int(account.get("imapPort") or 993)
    if account.get("imapStartTls"):
        box = imaplib.IMAP4(host, port, timeout=20)
        box.starttls(ssl.create_default_context())
        return box
    return imaplib.IMAP4_SSL(host, port, timeout=20)


def inbox(account, limit):
    box = connect_imap(account)
    try:
        box.login(account["username"], account["password"])
        box.select("INBOX", readonly=True)
        _, data = box.uid("SEARCH", None, "ALL")
        uids = (data[0] or b"").split()[-limit:]
        messages = []
        for uid in reversed(uids):
            _, parts = box.uid(
                "FETCH",
                uid,
                "(BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE MESSAGE-ID)])",
            )
            raw = b"".join(part[1] for part in parts if isinstance(part, tuple))
            if not raw:
                continue
            parsed = email.message_from_bytes(raw)
            messages.append(
                {
                    "uid": uid.decode(),
                    "from": decoded(parsed.get("From")),
                    "subject": decoded(parsed.get("Subject")),
                    "date": parsed.get("Date") or "",
                    "messageId": parsed.get("Message-ID") or "",
                }
            )
        return {"messages": messages}
    finally:
        try:
            box.logout()
        except Exception:
            pass


def send(account, request):
    message = EmailMessage()
    message["From"] = account.get("address") or account["username"]
    message["To"] = request["to"]
    message["Subject"] = request.get("subject") or "(no subject)"
    message.set_content(request.get("body") or "")
    host = account["smtpHost"]
    port = int(account.get("smtpPort") or 587)
    if account.get("smtpStartTls", True):
        client = smtplib.SMTP(host, port, timeout=30)
        client.starttls(context=ssl.create_default_context())
    else:
        client = smtplib.SMTP_SSL(host, port, timeout=30)
    try:
        client.login(account["username"], account["password"])
        client.send_message(message)
    finally:
        try:
            client.quit()
        except Exception:
            pass
    return {"sent": True}


def main():
    request = json.load(sys.stdin)
    account = request.get("account") or {}
    action = request.get("action")
    if action == "inbox":
        print(json.dumps(inbox(account, int(request.get("limit") or 20))))
    elif action == "send":
        print(json.dumps(send(account, request)))
    else:
        raise SystemExit(f"unknown email action: {action}")


if __name__ == "__main__":
    try:
        main()
    except Exception as failure:
        print(json.dumps({"error": str(failure)}))
        raise SystemExit(1)
