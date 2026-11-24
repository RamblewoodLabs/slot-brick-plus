#!/usr/bin/env python3
"""Home-LAN hub for Slot's continue-anywhere saves.

Files live under the hub root as files/<relative path>, with index.json beside
files/. A later iOS or Mac app can read that folder. This is not the old
download relay: different name, port, and directory.
"""
from __future__ import annotations

import argparse
import hashlib
import hmac
import json
import os
import socket
import struct
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, unquote, urlparse

PORT = 8791
MAX_BODY = 32 * 1024 * 1024
SYSTEMS = {"GB", "GBC", "GBA", "NES", "SNES", "PS1"}
CORES = {"mgba", "gpsp", "fceumm", "snes9x", "pcsx_rearmed"}
MDNS_NAME = b"_slotsync._tcp.local"


def decide(inc_mtime, inc_sha, inc_device, cur_mtime, cur_sha, cur_device):
    """Newest mtime wins. Identical bytes are left alone. Equal times break on device id."""
    if cur_sha and inc_sha == cur_sha:
        return "same"
    if cur_mtime is None:
        return "incoming"
    if inc_mtime > cur_mtime:
        return "incoming"
    if inc_mtime < cur_mtime:
        return "current"
    if inc_device == cur_device:
        return "incoming"
    return "incoming" if inc_device > cur_device else "current"


def path_ok(rel):
    if not rel or rel.startswith("/") or "\\" in rel or "\t" in rel or "\n" in rel:
        return False
    parts = rel.split("/")
    if any(part in ("", ".", "..") or part.startswith(".") for part in parts):
        return False
    if rel.endswith(".bak"):
        return False
    if rel in ("Config/library.txt", "Config/continue.txt", "Config/selected_core.ini", "Config/Achievements/queue.txt"):
        return True
    if rel.startswith("Config/Achievements/sets/"):
        name = rel[len("Config/Achievements/sets/") :]
        stem, dot, ext = name.rpartition(".")
        return dot == "." and ext in ("json", "list") and _key(stem) and "/" not in name
    if rel.startswith("Config/Achievements/sessions/"):
        name = rel[len("Config/Achievements/sessions/") :]
        stem, dot, ext = name.rpartition(".")
        return dot == "." and ext == "json" and stem.isdigit() and "/" not in name
    if rel.startswith("Saves/"):
        bits = rel.split("/")
        return len(bits) == 3 and bits[1] in SYSTEMS and _stem(bits[2]) and bits[2].rsplit(".", 1)[-1] in ("sav", "srm")
    if rel.startswith("States/"):
        bits = rel.split("/")
        if len(bits) != 5 or bits[1] not in SYSTEMS or bits[2] not in CORES or not _stem(bits[3]):
            return False
        leaf = bits[4]
        if leaf == "resume.state":
            return True
        stem, dot, ext = leaf.rpartition(".")
        if dot != "." or ext not in ("state", "png"):
            return False
        return _stamp(stem)
    return False


def _key(text):
    return 0 < len(text) <= 40 and text.isalnum()


def _stem(text):
    return (
        0 < len(text) <= 120
        and "/" not in text
        and not text.startswith(".")
        and all(ch not in text for ch in "\t\r\n\\")
    )


def _stamp(text):
    shape = "0000-00-00_00-00-00"
    if len(text) != len(shape):
        return False
    for got, want in zip(text, shape):
        if want == "0":
            if not got.isdigit():
                return False
        elif got != want:
            return False
    return True


class Store:
    def __init__(self, root: Path):
        self.root = root
        self.files = root / "files"
        self.index_path = root / "index.json"
        self.lock = threading.Lock()
        self.files.mkdir(parents=True, exist_ok=True)
        self.entries = {}
        if self.index_path.is_file():
            try:
                loaded = json.loads(self.index_path.read_text())
                self.entries = dict(loaded.get("files") or {})
            except (OSError, json.JSONDecodeError):
                self.entries = {}

    def index_rows(self):
        with self.lock:
            return [
                {
                    "path": path,
                    "mtime": int(meta["mtime"]),
                    "device": meta["device"],
                    "sha256": meta["sha256"],
                    "size": int(meta["size"]),
                }
                for path, meta in sorted(self.entries.items())
            ]

    def tsv(self):
        lines = []
        for row in self.index_rows():
            lines.append("{path}\t{mtime}\t{device}\t{sha256}\t{size}".format(**row))
        return ("\n".join(lines) + ("\n" if lines else "")).encode()

    def get(self, rel):
        with self.lock:
            meta = self.entries.get(rel)
            if meta is None:
                return None
            path = self._file(rel)
            try:
                data = path.read_bytes()
            except OSError:
                return None
            return data, dict(meta)

    def put(self, rel, data, mtime, device):
        digest = hashlib.sha256(data).hexdigest()
        with self.lock:
            current = self.entries.get(rel)
            choice = decide(
                mtime,
                digest,
                device,
                None if current is None else int(current["mtime"]),
                None if current is None else current["sha256"],
                None if current is None else current["device"],
            )
            if choice != "incoming":
                return "kept" if choice == "current" else "same"
            path = self._file(rel)
            path.parent.mkdir(parents=True, exist_ok=True)
            if path.is_file():
                bak = Path(str(path) + ".bak")
                if bak.is_file():
                    bak.unlink()
                os.replace(path, bak)
            tmp = path.with_name(path.name + ".tmp")
            tmp.write_bytes(data)
            os.replace(tmp, path)
            os.utime(path, ns=(mtime, mtime))
            self.entries[rel] = {
                "mtime": int(mtime),
                "device": device,
                "sha256": digest,
                "size": len(data),
            }
            self._write_index()
            return "stored"

    def _file(self, rel):
        path = (self.files / rel).resolve()
        root = self.files.resolve()
        if root != path and root not in path.parents:
            raise ValueError("path escapes the store")
        return path

    def _write_index(self):
        text = json.dumps({"version": 1, "files": self.entries}, indent=2, sort_keys=True)
        tmp = self.index_path.with_name("index.json.tmp")
        tmp.write_text(text + "\n")
        os.replace(tmp, self.index_path)


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.0"

    def log_message(self, fmt, *args):
        # The token arrives as a header. Log the method and path only, never headers.
        path = urlparse(self.path).path
        sys.stderr.write("slotsync: %s %s\n" % (self.command, path))

    def do_GET(self):
        if not self._auth():
            return
        parsed = urlparse(self.path)
        if parsed.path == "/v1/index":
            body = self.server.store.tsv()
            self._send(200, body, "text/plain")
            return
        if parsed.path == "/v1/file":
            rel = self._rel(parsed)
            if rel is None:
                return
            got = self.server.store.get(rel)
            if got is None:
                self._send(404, b"missing\n", "text/plain")
                return
            data, meta = got
            self.send_response(200)
            self.send_header("Content-Type", "application/octet-stream")
            self.send_header("Content-Length", str(len(data)))
            self.send_header("X-Slot-Mtime", str(meta["mtime"]))
            self.send_header("X-Slot-Device", meta["device"])
            self.send_header("X-Slot-Sha256", meta["sha256"])
            self.end_headers()
            self.wfile.write(data)
            return
        self._send(404, b"missing\n", "text/plain")

    def do_PUT(self):
        if not self._auth():
            return
        parsed = urlparse(self.path)
        if parsed.path != "/v1/file":
            self._send(404, b"missing\n", "text/plain")
            return
        rel = self._rel(parsed)
        if rel is None:
            return
        try:
            length = int(self.headers.get("Content-Length", "0"))
            mtime = int(self.headers.get("X-Slot-Mtime", "0"))
        except ValueError:
            self._send(400, b"bad\n", "text/plain")
            return
        device = self.headers.get("X-Slot-Device", "")
        if length < 0 or length > MAX_BODY or mtime < 0 or not _device(device):
            self._send(400, b"bad\n", "text/plain")
            return
        data = self.rfile.read(length)
        if len(data) != length:
            self._send(400, b"short\n", "text/plain")
            return
        result = self.server.store.put(rel, data, mtime, device)
        self._send(200, (result + "\n").encode(), "text/plain")

    def _rel(self, parsed):
        query = parse_qs(parsed.query)
        raw = query.get("path", [""])[0]
        rel = unquote(raw)
        if not path_ok(rel):
            self._send(400, b"bad path\n", "text/plain")
            return None
        return rel

    def _auth(self):
        header = self.headers.get("Authorization", "")
        token = header[7:] if header.startswith("Bearer ") else ""
        if hmac.compare_digest(token, self.server.token):
            return True
        self._send(401, b"unauthorized\n", "text/plain")
        return False

    def _send(self, status, body, content_type):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def _device(text):
    return 0 < len(text) <= 32 and all(ch.isalnum() or ch == "-" for ch in text)


def serve(root, token, port, bind):
    store = Store(Path(root))
    httpd = ThreadingHTTPServer((bind, port), Handler)
    httpd.store = store
    httpd.token = token
    return httpd


def advertise(port, stop):
    """Answer _slotsync._tcp.local on the LAN. Failure to bind is not fatal."""
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(("", 5353))
        group = socket.inet_aton("224.0.0.251")
        sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, group + socket.inet_aton("0.0.0.0"))
        sock.settimeout(0.5)
    except OSError as exc:
        sys.stderr.write("slotsync: mDNS off (%s)\n" % exc)
        return
    name = socket.gethostname().split(".")[0] + ".local"
    while not stop.is_set():
        try:
            data, addr = sock.recvfrom(1500)
        except socket.timeout:
            continue
        except OSError:
            break
        if MDNS_NAME not in data:
            continue
        try:
            packet = mdns_reply(name, port)
            sock.sendto(packet, addr)
        except OSError:
            continue
    sock.close()


def mdns_reply(host, port):
    """A tiny PTR + SRV + A reply. host is 'Name.local'."""
    def label(text):
        out = b""
        for part in text.rstrip(".").split("."):
            raw = part.encode()
            out += bytes([len(raw)]) + raw
        return out + b"\x00"

    question = label("_slotsync._tcp.local")
    header = struct.pack(">HHHHHH", 0, 0x8400, 0, 1, 0, 0)
    answer = question + struct.pack(">HHIH", 12, 1, 120, len(label(host)))
    # PTR target is the host name; good enough for a client that only needs an address.
    # A real SRV is below, and the A record uses the first non-loopback IPv4 we can see.
    ptr = question + struct.pack(">HHI", 12, 1, 120) + _len_bytes(label("slot." + host))
    ip = _lan_ip()
    a_name = label(host)
    a = a_name + struct.pack(">HHIH", 1, 1, 120, 4) + socket.inet_aton(ip)
    srv_target = label(host)
    srv_rdata = struct.pack(">HHH", 0, 0, port) + srv_target
    srv = question + struct.pack(">HHIH", 33, 1, 120, len(srv_rdata)) + srv_rdata
    header = struct.pack(">HHHHHH", 0, 0x8400, 0, 3, 0, 0)
    return header + ptr + srv + a


def _len_bytes(payload):
    return struct.pack(">H", len(payload)) + payload


def _lan_ip():
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.connect(("192.168.7.99", 9))
        ip = sock.getsockname()[0]
        sock.close()
        return ip
    except OSError:
        return "0.0.0.0"


def load_config(root):
    path = Path(root) / "config.json"
    data = json.loads(path.read_text())
    token = str(data.get("token") or "")
    port = int(data.get("port") or PORT)
    bind = str(data.get("bind") or "0.0.0.0")
    if not token:
        raise SystemExit("slotsync: config.json needs a token")
    return token, port, bind


def main(argv=None):
    parser = argparse.ArgumentParser(description="Slot continue-anywhere hub")
    parser.add_argument("--root", required=True)
    parser.add_argument("--port", type=int)
    parser.add_argument("--bind", default=None)
    parser.add_argument("--token", default=None)
    parser.add_argument("--no-mdns", action="store_true")
    args = parser.parse_args(argv)
    root = Path(args.root)
    root.mkdir(parents=True, exist_ok=True)
    if args.token:
        token = args.token
        port = PORT if args.port is None else args.port
        bind = args.bind or "127.0.0.1"
    else:
        token, port, bind = load_config(root)
        if args.port is not None:
            port = args.port
        if args.bind:
            bind = args.bind
    httpd = serve(root, token, port, bind)
    print("port=%s" % httpd.server_address[1], flush=True)
    stop = threading.Event()
    if not args.no_mdns:
        threading.Thread(target=advertise, args=(httpd.server_address[1], stop), daemon=True).start()
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        stop.set()
        httpd.server_close()


if __name__ == "__main__":
    main()
