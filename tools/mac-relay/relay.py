#!/usr/bin/env python3
"""Home download relay for Slot on the TrimUI Brick.

The Mac fetches a YouTube video or its audio with yt-dlp, in the H.264+AAC
MP4 (or AAC M4A) the Brick already plays, and the Brick copies the file over
the LAN. The file is deleted when the Brick confirms it, or after 24 hours.
"""

from __future__ import annotations

import argparse
import hashlib
import hmac
import json
import os
import re
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

KEEP_SECONDS = 24 * 60 * 60
VIDEO_ID = re.compile(r"^[A-Za-z0-9_-]{6,16}$")
JOB_ID = re.compile(r"^(video|audio)-[A-Za-z0-9_-]{6,16}$")
PROGRESS = re.compile(
    r"\[download\]\s+([\d.]+)%(?:\s+of\s+~?\s*([\d.]+)\s*([KMG]i?B))?"
)


def job_id(kind: str, video_id: str) -> str:
    return f"{kind}-{video_id}"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _bytes_of(amount: str, unit: str) -> int:
    scale = 1
    unit = unit.upper().replace("I", "")
    if unit.startswith("K"):
        scale = 1024
    elif unit.startswith("M"):
        scale = 1024 * 1024
    elif unit.startswith("G"):
        scale = 1024 * 1024 * 1024
    return int(float(amount) * scale)


class Relay:
    def __init__(
        self,
        root: Path,
        token: str,
        ytdlp: str = "yt-dlp",
        ffmpeg: str = "",
        keep_seconds: int = KEEP_SECONDS,
        now=None,
    ):
        self.root = Path(root)
        self.root.mkdir(parents=True, exist_ok=True)
        self.token = token
        self.ytdlp = ytdlp
        self.ffmpeg = ffmpeg
        self.keep_seconds = keep_seconds
        self.now = now or time.time
        self.lock = threading.Lock()
        self.jobs: dict[str, dict] = {}
        self._load()
        self.sweep()

    def _path(self, name: str) -> Path:
        return self.root / name

    def _load(self) -> None:
        for path in self.root.glob("*.json"):
            try:
                job = json.loads(path.read_text())
            except (OSError, json.JSONDecodeError):
                continue
            if not JOB_ID.match(str(job.get("id", ""))):
                continue
            if job.get("status") in ("queued", "downloading"):
                job["status"] = "failed"
                job["error"] = "interrupted"
                self._write(job)
            self.jobs[job["id"]] = job

    def _write(self, job: dict) -> None:
        tmp = self._path(job["id"] + ".json.tmp")
        tmp.write_text(json.dumps(job))
        tmp.replace(self._path(job["id"] + ".json"))

    def sweep(self) -> int:
        removed = 0
        now = self.now()
        with self.lock:
            for job in list(self.jobs.values()):
                ready_at = job.get("ready_at") or 0
                if job.get("status") == "ready" and now - ready_at > self.keep_seconds:
                    self._drop(job["id"])
                    removed += 1
            known = set()
            for job in self.jobs.values():
                if job.get("path"):
                    known.add(Path(job["path"]).name)
                known.add(job["id"] + ".json")
            for path in self.root.iterdir():
                if path.name in known or path.suffix == ".json":
                    continue
                if path.is_file() and now - path.stat().st_mtime > self.keep_seconds:
                    path.unlink(missing_ok=True)
                    removed += 1
        return removed

    def _drop(self, job_id: str) -> None:
        job = self.jobs.pop(job_id, None)
        self._path(job_id + ".json").unlink(missing_ok=True)
        if job and job.get("path"):
            Path(job["path"]).unlink(missing_ok=True)
        work = self.root / job_id
        if work.is_dir():
            for child in work.iterdir():
                child.unlink(missing_ok=True)
            work.rmdir()

    def authorized(self, header: str, query_token: str) -> bool:
        expected = f"Bearer {self.token}".encode()
        given = header.encode()
        if len(given) == len(expected) and hmac.compare_digest(given, expected):
            return True
        other = query_token.encode()
        token = self.token.encode()
        return len(other) == len(token) and hmac.compare_digest(other, token)

    def submit(self, video_id: str, kind: str) -> dict:
        if kind not in ("video", "audio") or not VIDEO_ID.match(video_id):
            raise ValueError("bad video")
        ident = job_id(kind, video_id)
        with self.lock:
            job = self.jobs.get(ident)
            if job and job.get("status") in ("queued", "downloading", "ready"):
                return self.public(job)
            job = {
                "id": ident,
                "video_id": video_id,
                "kind": kind,
                "status": "queued",
                "bytes": 0,
                "total": 0,
                "size": 0,
                "sha256": "",
                "error": "",
                "path": "",
                "ready_at": 0,
                "cancel": False,
            }
            self.jobs[ident] = job
            self._write(job)
        threading.Thread(target=self._run, args=(ident,), daemon=True).start()
        return self.public(job)

    def public(self, job: dict) -> dict:
        return {
            "id": job["id"],
            "status": job["status"],
            "bytes": job.get("bytes") or 0,
            "total": job.get("total") or 0,
            "size": job.get("size") or 0,
            "sha256": job.get("sha256") or "",
            "error": job.get("error") or "",
        }

    def get(self, ident: str) -> dict | None:
        with self.lock:
            job = self.jobs.get(ident)
            return self.public(job) if job else None

    def file_of(self, ident: str) -> tuple[Path, str] | None:
        with self.lock:
            job = self.jobs.get(ident)
            if not job or job.get("status") != "ready" or not job.get("path"):
                return None
            path = Path(job["path"])
            digest = job.get("sha256") or ""
        if not path.is_file():
            return None
        return path, digest

    def delete(self, ident: str) -> bool:
        with self.lock:
            job = self.jobs.get(ident)
            if job:
                job["cancel"] = True
                self._write(job)
            self._drop(ident)
        return True

    def _update(self, ident: str, **fields) -> None:
        with self.lock:
            job = self.jobs.get(ident)
            if not job or job.get("cancel"):
                return
            job.update(fields)
            self._write(job)

    def _cancelled(self, ident: str) -> bool:
        with self.lock:
            job = self.jobs.get(ident)
            return not job or bool(job.get("cancel"))

    def _run(self, ident: str) -> None:
        with self.lock:
            job = dict(self.jobs.get(ident) or {})
        if not job:
            return
        self._update(ident, status="downloading")
        work = self.root / ident
        work.mkdir(exist_ok=True)
        target = work / ("download.mp4" if job["kind"] == "video" else "download.m4a")
        url = f"https://www.youtube.com/watch?v={job['video_id']}"
        try:
            self._download(ident, job["kind"], url, target)
            if self._cancelled(ident):
                return
            final = self._normalize(target, job["kind"])
            if self._cancelled(ident):
                return
            digest = sha256_file(final)
            size = final.stat().st_size
            kept = self.root / (ident + final.suffix)
            if final.resolve() != kept.resolve():
                final.replace(kept)
            self._update(
                ident,
                status="ready",
                bytes=size,
                total=size,
                size=size,
                sha256=digest,
                path=str(kept),
                ready_at=self.now(),
                error="",
            )
        except Exception as exc:  # noqa: BLE001 — the Brick sees the message
            if not self._cancelled(ident):
                self._update(ident, status="failed", error=str(exc)[:300])

    def _download(self, ident: str, kind: str, url: str, target: Path) -> None:
        if kind == "video":
            formats = [
                ["-f", "18", "--merge-output-format", "mp4"],
                [
                    "-f",
                    "bv*[height<=360][vcodec^=avc1]+ba[acodec^=mp4a]/b[height<=360][ext=mp4]/b",
                    "--merge-output-format",
                    "mp4",
                ],
            ]
        else:
            formats = [
                ["-f", "140/ba[acodec^=mp4a]/ba", "-x", "--audio-format", "m4a"],
            ]
        last = "yt-dlp failed"
        for extra in formats:
            if target.exists():
                target.unlink()
            cmd = [
                self.ytdlp,
                "--no-playlist",
                "--newline",
                "--no-colors",
                "-o",
                str(target),
                *extra,
                url,
            ]
            if self.ffmpeg:
                cmd[1:1] = ["--ffmpeg-location", str(Path(self.ffmpeg).parent)]
            try:
                self._run_ytdlp(ident, cmd)
            except RuntimeError as exc:
                last = str(exc)
                continue
            if target.is_file() and target.stat().st_size > 0:
                return
            produced = [p for p in target.parent.iterdir() if p.is_file() and p.stat().st_size > 0]
            if produced:
                produced.sort(key=lambda p: p.stat().st_mtime, reverse=True)
                produced[0].replace(target)
                return
            last = "yt-dlp wrote no file"
        raise RuntimeError(last)

    def _run_ytdlp(self, ident: str, cmd: list[str]) -> None:
        proc = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        assert proc.stdout is not None
        try:
            for line in proc.stdout:
                if self._cancelled(ident):
                    proc.kill()
                    proc.wait(timeout=5)
                    return
                self._note_progress(ident, line)
            code = proc.wait()
        finally:
            proc.stdout.close()
        if code != 0:
            raise RuntimeError(f"yt-dlp exited {code}")

    def _note_progress(self, ident: str, line: str) -> None:
        line = line.strip()
        if line.startswith("PROGRESS "):
            parts = line.split()
            if len(parts) >= 3 and parts[1].isdigit() and parts[2].isdigit():
                self._update(ident, bytes=int(parts[1]), total=int(parts[2]))
            return
        match = PROGRESS.search(line)
        if not match:
            return
        percent = float(match.group(1))
        total = _bytes_of(match.group(2), match.group(3)) if match.group(2) else 0
        got = int(total * percent / 100) if total else int(percent)
        self._update(ident, bytes=got, total=total)

    def _normalize(self, src: Path, kind: str) -> Path:
        if not self.ffmpeg or not Path(self.ffmpeg).is_file():
            return src
        if kind == "video" and self._already_brick(src):
            return src
        dest = src.with_name(src.stem + ".brick" + (".m4a" if kind == "audio" else ".mp4"))
        if kind == "audio":
            cmd = [
                self.ffmpeg, "-y", "-i", str(src),
                "-vn", "-c:a", "aac", "-ac", "2", "-b:a", "128k",
                str(dest),
            ]
        else:
            cmd = [
                self.ffmpeg, "-y", "-i", str(src),
                "-c:v", "libx264", "-profile:v", "baseline", "-level", "3.0",
                "-pix_fmt", "yuv420p", "-vf", "scale=-2:'min(360,ih)'",
                "-c:a", "aac", "-ac", "2", "-b:a", "128k",
                "-movflags", "+faststart", str(dest),
            ]
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        src.unlink(missing_ok=True)
        return dest

    def _already_brick(self, src: Path) -> bool:
        probe = Path(self.ffmpeg).with_name("ffprobe")
        if not probe.is_file():
            return False
        try:
            out = subprocess.check_output(
                [
                    str(probe), "-v", "error",
                    "-show_entries", "stream=codec_name,height,codec_type",
                    "-of", "csv=p=0", str(src),
                ],
                text=True,
                stderr=subprocess.DEVNULL,
            )
        except (OSError, subprocess.CalledProcessError):
            return False
        video = audio = False
        for line in out.splitlines():
            cells = [c.strip() for c in line.split(",")]
            if "h264" in cells and "video" in cells:
                height = next((int(c) for c in cells if c.isdigit()), 0)
                video = height == 0 or height <= 360
            if "aac" in cells and "audio" in cells:
                audio = True
        return video and audio


class Handler(BaseHTTPRequestHandler):
    relay: Relay
    protocol_version = "HTTP/1.0"

    def log_message(self, fmt: str, *args) -> None:
        return

    def _token(self) -> str:
        query = parse_qs(urlsplit(self.path).query)
        return (query.get("token") or [""])[0]

    def _deny(self) -> bool:
        header = self.headers.get("Authorization", "")
        if self.relay.authorized(header, self._token()):
            return False
        self._json(401, {"error": "unauthorized"})
        return True

    def _json(self, status: int, body: dict) -> None:
        raw = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(raw)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(raw)

    def do_POST(self) -> None:  # noqa: N802
        if self._deny():
            return
        path = urlsplit(self.path).path
        if path != "/jobs":
            self._json(404, {"error": "not found"})
            return
        length = int(self.headers.get("Content-Length", "0") or 0)
        if length > 4096:
            self._json(400, {"error": "bad video"})
            return
        try:
            payload = json.loads(self.rfile.read(length) or b"{}")
            body = self.relay.submit(str(payload.get("video_id", "")), str(payload.get("kind", "")))
        except (ValueError, json.JSONDecodeError):
            self._json(400, {"error": "bad video"})
            return
        self._json(202, body)

    def do_GET(self) -> None:  # noqa: N802
        if self._deny():
            return
        path = urlsplit(self.path).path
        if path.startswith("/jobs/"):
            ident = path[len("/jobs/"):]
            if not JOB_ID.match(ident):
                self._json(404, {"error": "not found"})
                return
            job = self.relay.get(ident)
            if not job:
                self._json(404, {"error": "not found"})
                return
            self._json(200, job)
            return
        if path.startswith("/files/"):
            ident = path[len("/files/"):]
            found = self.relay.file_of(ident) if JOB_ID.match(ident) else None
            if not found:
                self._json(404, {"error": "not found"})
                return
            self._send_file(*found)
            return
        self._json(404, {"error": "not found"})

    def do_DELETE(self) -> None:  # noqa: N802
        if self._deny():
            return
        path = urlsplit(self.path).path
        if not path.startswith("/files/"):
            self._json(404, {"error": "not found"})
            return
        ident = path[len("/files/"):]
        if not JOB_ID.match(ident):
            self._json(404, {"error": "not found"})
            return
        self.relay.delete(ident)
        self.send_response(204)
        self.send_header("Content-Length", "0")
        self.send_header("Connection", "close")
        self.end_headers()

    def _send_file(self, path: Path, digest: str) -> None:
        size = path.stat().st_size
        start, end = 0, size - 1
        status = 200
        header = self.headers.get("Range")
        if header:
            parsed = _parse_range(header, size)
            if parsed is None:
                self.send_response(416)
                self.send_header("Content-Range", f"bytes */{size}")
                self.send_header("Content-Length", "0")
                self.send_header("Connection", "close")
                self.end_headers()
                return
            start, end = parsed
            status = 206
        length = end - start + 1
        self.send_response(status)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Length", str(length))
        self.send_header("X-Sha256", digest)
        if status == 206:
            self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
        self.send_header("Connection", "close")
        self.end_headers()
        with path.open("rb") as handle:
            handle.seek(start)
            remaining = length
            while remaining:
                chunk = handle.read(min(remaining, 1024 * 1024))
                if not chunk:
                    break
                self.wfile.write(chunk)
                remaining -= len(chunk)


def _parse_range(header: str, size: int) -> tuple[int, int] | None:
    if not header.startswith("bytes="):
        return None
    spec = header.split("=", 1)[1].split(",", 1)[0].strip()
    start_s, _, end_s = spec.partition("-")
    try:
        start = int(start_s) if start_s else 0
        end = int(end_s) if end_s else size - 1
    except ValueError:
        return None
    if start < 0 or start >= size or end < start:
        return None
    return start, min(end, size - 1)


def load_config(path: Path) -> dict:
    data = json.loads(path.read_text())
    data.setdefault("keep_seconds", KEEP_SECONDS)
    data.setdefault("bind", "0.0.0.0")
    data.setdefault("port", 8787)
    data.setdefault("ytdlp", "yt-dlp")
    data.setdefault("ffmpeg", "")
    return data


def serve(config: dict) -> ThreadingHTTPServer:
    relay = Relay(
        root=Path(config["dir"]).expanduser(),
        token=config["token"],
        ytdlp=config.get("ytdlp") or "yt-dlp",
        ffmpeg=config.get("ffmpeg") or "",
        keep_seconds=int(config.get("keep_seconds") or KEEP_SECONDS),
    )
    Handler.relay = relay
    server = ThreadingHTTPServer((config.get("bind") or "0.0.0.0", int(config["port"])), Handler)
    threading.Thread(target=_sweeper, args=(relay,), daemon=True).start()
    return server


def _sweeper(relay: Relay) -> None:
    while True:
        time.sleep(60 * 60)
        relay.sweep()


def main() -> None:
    parser = argparse.ArgumentParser(description="Slot home download relay")
    parser.add_argument("--config", required=True)
    args = parser.parse_args()
    config = load_config(Path(args.config).expanduser())
    if not config.get("token"):
        raise SystemExit("config token is empty")
    server = serve(config)
    host, port = server.server_address[:2]
    print(f"slot relay on {host}:{port}", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
