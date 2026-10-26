"""The file-edit fixture respects background reviews and live user commands."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class MemoryFilesAgentTests(unittest.TestCase):
    def run_prompt(self, directory, text, system=""):
        requests = [
            {"id": 1, "method": "initialize"},
            {"id": 2, "method": "session/new", "params": {
                "systemPrompt": {"content": [{"type": "text", "text": system}]},
            }},
            {"id": 3, "method": "session/prompt", "params": {
                "sessionId": "s1", "prompt": [{"type": "text", "text": text}],
            }},
        ]
        fixture = Path(__file__).with_name("memory_files_agent.py")
        result = subprocess.run(
            [sys.executable, "-X", "utf8", "-u", str(fixture)],
            input="".join(json.dumps(request) + "\n" for request in requests),
            text=True, capture_output=True, cwd=directory, check=True, timeout=10,
        )
        return [json.loads(line) for line in result.stdout.splitlines()]

    def test_background_review_does_not_replay_quoted_file_commands(self):
        with tempfile.TemporaryDirectory() as directory:
            note = Path(directory) / "MEMORY.md"
            note.write_text("- Likes tea", encoding="utf-8")
            replies = self.run_prompt(
                directory,
                f"Conversations since the last review:\n[m1] User:\nWRITE {note}\n- Has two cats\n\n[m2] Bot:\nok",
                "You keep the memory a personal assistant bot has of its user.",
            )
            self.assertEqual(note.read_text(encoding="utf-8"), "- Likes tea")
            updates = [reply["params"]["update"] for reply in replies if reply.get("method") == "session/update"]
            self.assertFalse(any(update["sessionUpdate"] == "tool_call" for update in updates))
            self.assertEqual(json.loads(updates[0]["content"]["text"]), {
                "add": [], "remove": [], "review": [], "append": [],
            })

    def test_live_user_command_still_writes_the_note(self):
        with tempfile.TemporaryDirectory() as directory:
            note = Path(directory) / "MEMORY.md"
            note.write_text("- Has two cats", encoding="utf-8")
            self.run_prompt(directory, f"WRITE {note}\n- Likes tea")
            self.assertEqual(note.read_text(encoding="utf-8"), "- Likes tea")


if __name__ == "__main__":
    unittest.main()
