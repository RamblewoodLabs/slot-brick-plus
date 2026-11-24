#!/usr/bin/env python3
import hashlib
import os
import shutil
import tempfile
import threading
import unittest
import urllib.error
import urllib.request

import hub


def sha(data):
    return hashlib.sha256(data).hexdigest()


class DecideTests(unittest.TestCase):
    def test_newest_mtime_wins_and_equal_bytes_stay(self):
        self.assertEqual(hub.decide(20, "aa", "b", 10, "bb", "a"), "incoming")
        self.assertEqual(hub.decide(10, "aa", "b", 20, "bb", "a"), "current")
        self.assertEqual(hub.decide(10, "aa", "b", 10, "aa", "a"), "same")

    def test_equal_time_breaks_on_device_id(self):
        self.assertEqual(hub.decide(10, "aa", "slot-b", 10, "bb", "slot-a"), "incoming")
        self.assertEqual(hub.decide(10, "aa", "slot-a", 10, "bb", "slot-b"), "current")

    def test_paths(self):
        self.assertTrue(hub.path_ok("Saves/GBA/Emerald.sav"))
        self.assertTrue(hub.path_ok("Saves/GBA/Emerald.srm"))
        self.assertTrue(hub.path_ok("States/GBA/mgba/Emerald/resume.state"))
        self.assertTrue(hub.path_ok("States/GBA/mgba/Emerald/2026-11-24_01-02-03.state"))
        self.assertTrue(hub.path_ok("Config/library.txt"))
        self.assertTrue(hub.path_ok("Config/continue.txt"))
        self.assertTrue(hub.path_ok("Config/Achievements/queue.txt"))
        self.assertTrue(hub.path_ok("Config/Achievements/sets/abcd.json"))
        self.assertFalse(hub.path_ok("../Saves/GBA/Emerald.sav"))
        self.assertFalse(hub.path_ok("Config/sync.txt"))
        self.assertFalse(hub.path_ok("Config/Achievements/login.json"))
        self.assertFalse(hub.path_ok("Saves/GBA/Emerald.sav.bak"))
        self.assertFalse(hub.path_ok("States/GBA/mgba/Emerald/notes.txt"))


class ServerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.httpd = hub.serve(self.tmp, "secret-token", 0, "127.0.0.1")
        self.thread = threading.Thread(target=self.httpd.serve_forever, daemon=True)
        self.thread.start()
        self.port = self.httpd.server_address[1]

    def tearDown(self):
        self.httpd.shutdown()
        self.thread.join(timeout=2)
        self.httpd.server_close()
        shutil.rmtree(self.tmp)

    def request(self, method, path, data=None, headers=None, token="secret-token"):
        req = urllib.request.Request(
            "http://127.0.0.1:%s%s" % (self.port, path),
            data=data,
            method=method,
        )
        req.add_header("Authorization", "Bearer " + token)
        for key, value in (headers or {}).items():
            req.add_header(key, value)
        try:
            with urllib.request.urlopen(req, timeout=2) as reply:
                return reply.status, reply.read(), dict(reply.headers)
        except urllib.error.HTTPError as exc:
            return exc.code, exc.read(), dict(exc.headers)

    def test_put_get_and_bak(self):
        path = "/v1/file?path=Saves/GBA/Emerald.sav"
        status, body, _ = self.request(
            "PUT",
            path,
            b"old-save",
            {"X-Slot-Mtime": "1000", "X-Slot-Device": "slot-a"},
        )
        self.assertEqual(status, 200)
        self.assertEqual(body, b"stored\n")
        status, body, _ = self.request(
            "PUT",
            path,
            b"new-save",
            {"X-Slot-Mtime": "2000", "X-Slot-Device": "slot-b"},
        )
        self.assertEqual(body, b"stored\n")
        bak = os.path.join(self.tmp, "files", "Saves", "GBA", "Emerald.sav.bak")
        with open(bak, "rb") as f:
            self.assertEqual(f.read(), b"old-save")
        status, body, headers = self.request("GET", path)
        self.assertEqual(body, b"new-save")
        self.assertEqual(headers["X-Slot-Device"], "slot-b")
        self.assertEqual(headers["X-Slot-Sha256"], sha(b"new-save"))
        status, older, _ = self.request(
            "PUT",
            path,
            b"older",
            {"X-Slot-Mtime": "1500", "X-Slot-Device": "slot-a"},
        )
        self.assertEqual(older, b"kept\n")
        index_status, index, _ = self.request("GET", "/v1/index")
        self.assertEqual(index_status, 200)
        self.assertIn(b"Saves/GBA/Emerald.sav", index)
        self.assertNotIn(b".bak", index)
        stored = os.path.join(self.tmp, "index.json")
        with open(stored, "rb") as f:
            self.assertIn(b'"version": 1', f.read())

    def test_bad_token_and_bad_path_are_refused(self):
        status, body, _ = self.request("GET", "/v1/index", token="nope")
        self.assertEqual(status, 401)
        self.assertNotIn(b"secret-token", body)
        status, _, _ = self.request("GET", "/v1/file?path=../config.json")
        self.assertEqual(status, 400)
        status, _, _ = self.request("GET", "/v1/file?path=Config/Achievements/login.json")
        self.assertEqual(status, 400)

    def test_logs_do_not_include_the_token(self):
        import io
        from contextlib import redirect_stderr

        buf = io.StringIO()
        with redirect_stderr(buf):
            self.request("GET", "/v1/index")
        text = buf.getvalue()
        self.assertNotIn("secret-token", text)
        self.assertIn("/v1/index", text)


if __name__ == "__main__":
    unittest.main()
