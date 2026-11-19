#!/usr/bin/env python3
"""Job lifecycle, Range, delete-after-confirm, and 24 hour expiry."""

import hashlib
import json
import threading
import time
import unittest
import urllib.error
import urllib.request
from pathlib import Path
from tempfile import TemporaryDirectory

import relay


BODY = b"abcdefghijklmnopqrstuvwxyz0123456789"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class RelayTest(unittest.TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.ytdlp = self.root / "ytdlp"
        self.ytdlp.write_text(
            "#!/bin/sh\n"
            "out=\n"
            "prev=\n"
            "for arg in \"$@\"; do\n"
            "  if [ \"$prev\" = \"-o\" ]; then out=$arg; fi\n"
            "  prev=$arg\n"
            "done\n"
            "echo PROGRESS 4 36\n"
            "echo PROGRESS 36 36\n"
            "printf '%s' 'abcdefghijklmnopqrstuvwxyz0123456789' > \"$out\"\n"
        )
        self.ytdlp.chmod(0o755)
        self.relay = relay.Relay(
            root=self.root / "data",
            token="secret-token",
            ytdlp=str(self.ytdlp),
            ffmpeg="",
            keep_seconds=relay.KEEP_SECONDS,
        )
        relay.Handler.relay = self.relay
        self.server = relay.ThreadingHTTPServer(("127.0.0.1", 0), relay.Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.port = self.server.server_address[1]

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.tmp.cleanup()

    def request(self, method, path, payload=None, token="secret-token", headers=None):
        data = None if payload is None else json.dumps(payload).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{self.port}{path}",
            data=data,
            method=method,
        )
        req.add_header("Authorization", f"Bearer {token}")
        if data is not None:
            req.add_header("Content-Type", "application/json")
        for key, value in (headers or {}).items():
            req.add_header(key, value)
        try:
            with urllib.request.urlopen(req, timeout=5) as reply:
                raw = reply.read()
                return reply.status, dict(reply.headers), raw
        except urllib.error.HTTPError as exc:
            return exc.code, dict(exc.headers), exc.read()

    def test_job_range_and_delete(self):
        status, _, raw = self.request(
            "POST", "/jobs", {"video_id": "abcdefghijk", "kind": "video"}
        )
        self.assertEqual(status, 202)
        posted = json.loads(raw)
        self.assertEqual(posted["id"], "video-abcdefghijk")
        ready = None
        for _ in range(50):
            status, _, raw = self.request("GET", "/jobs/video-abcdefghijk")
            self.assertEqual(status, 200)
            job = json.loads(raw)
            if job["status"] == "ready":
                ready = job
                break
            if job["status"] == "failed":
                self.fail(job["error"])
        self.assertIsNotNone(ready)
        self.assertEqual(ready["size"], len(BODY))
        self.assertEqual(ready["sha256"], sha(BODY))

        status, headers, raw = self.request(
            "GET",
            "/files/video-abcdefghijk",
            headers={"Range": "bytes=0-9"},
        )
        self.assertEqual(status, 206)
        self.assertEqual(raw, BODY[:10])
        self.assertEqual(headers["Content-Length"], "10")
        self.assertEqual(headers["X-Sha256"], sha(BODY))
        self.assertIn("bytes 0-9/", headers["Content-Range"])

        status, headers, raw = self.request(
            "GET",
            "/files/video-abcdefghijk",
            headers={"Range": "bytes=10-"},
        )
        self.assertEqual(status, 206)
        self.assertEqual(raw, BODY[10:])
        self.assertEqual(int(headers["Content-Length"]), len(BODY) - 10)

        status, _, _ = self.request("DELETE", "/files/video-abcdefghijk")
        self.assertEqual(status, 204)
        status, _, _ = self.request("GET", "/files/video-abcdefghijk")
        self.assertEqual(status, 404)
        status, _, _ = self.request("GET", "/jobs/video-abcdefghijk")
        self.assertEqual(status, 404)
        self.assertFalse(any((self.root / "data").glob("video-abcdefghijk*")))

    def test_a_wrong_token_is_refused(self):
        status, _, _ = self.request(
            "POST", "/jobs", {"video_id": "abcdefghijk", "kind": "audio"}, token="nope"
        )
        self.assertEqual(status, 401)

    def test_a_file_not_collected_within_a_day_is_deleted(self):
        self.relay.keep_seconds = relay.KEEP_SECONDS
        posted = self.relay.submit("zzzzzzzzzzz", "audio")
        job = {"status": "queued"}
        for _ in range(50):
            job = self.relay.get(posted["id"])
            if job["status"] in ("ready", "failed"):
                break
            time.sleep(0.05)
        self.assertEqual(self.relay.get(posted["id"])["status"], "ready")
        stored = self.relay.jobs[posted["id"]]
        stored["ready_at"] = self.relay.now() - relay.KEEP_SECONDS - 1
        self.relay._write(stored)
        path = Path(stored["path"])
        self.assertTrue(path.is_file())
        self.assertGreater(self.relay.sweep(), 0)
        self.assertFalse(path.exists())
        self.assertIsNone(self.relay.get(posted["id"]))

    def test_startup_removes_an_expired_file(self):
        data = self.root / "fresh"
        data.mkdir()
        ident = "video-abcdefghijk"
        payload = BODY
        media = data / f"{ident}.mp4"
        media.write_bytes(payload)
        record = {
            "id": ident,
            "video_id": "abcdefghijk",
            "kind": "video",
            "status": "ready",
            "bytes": len(payload),
            "total": len(payload),
            "size": len(payload),
            "sha256": sha(payload),
            "error": "",
            "path": str(media),
            "ready_at": time.time() - relay.KEEP_SECONDS - 5,
            "cancel": False,
        }
        (data / f"{ident}.json").write_text(json.dumps(record))
        started = relay.Relay(root=data, token="secret-token", ytdlp=str(self.ytdlp), ffmpeg="")
        self.assertFalse(media.exists())
        self.assertIsNone(started.get(ident))


if __name__ == "__main__":
    unittest.main()
