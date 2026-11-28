#!/usr/bin/env python3
"""Fetch ScreenScraper media for the cart-art test set (test tooling, not shipped).

    python3 tools/cartart/fetch_ss.py --creds android/screenscraper.properties \
        --games tools/cartart/games.tsv --out ~/Shared-Space/cartart-test

Developer-only access: 1 thread, ~10k requests a day, 128 KB/s. Calls are made one at a
time with a pause between them, and everything already on disk is skipped. Media URLs carry
the credentials, so no URL is ever printed or written: only type and region.
"""
import argparse, json, pathlib, re, sys, time, urllib.parse, urllib.request

API = "https://api.screenscraper.fr/api2/"
PAUSE = 1.5
UA = "Slotted-cartart/1"
# What the engine can use, best first. Arcade adds marquees.
TYPES = ["support-2D", "support-texture", "box-2D", "wheel-hd", "wheel", "screenmarquee", "marquee"]
MAXW = 1200


def redact(s):
    return re.sub(r"(devid|devpassword|ssid|sspassword)=[^&\s]*", r"\1=***", str(s))


def creds(path):
    out = {}
    for line in pathlib.Path(path).expanduser().read_text().splitlines():
        if "=" in line and not line.lstrip().startswith("#"):
            k, v = line.split("=", 1)
            out[k.strip()] = v.strip()
    return {k: out[k] for k in ("devid", "devpassword", "softname")}


last = [0.0]


def get(url):
    wait = last[0] + PAUSE - time.time()
    if wait > 0:
        time.sleep(wait)
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": UA}), timeout=120) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()
    except Exception as e:  # noqa
        return 0, redact(e).encode()
    finally:
        last[0] = time.time()


def regions_for(rom):
    tags = " ".join(re.findall(r"\(([^)]*)\)", rom))
    pref = []
    for word, reg in (("USA", "us"), ("World", "wor"), ("Europe", "eu"), ("Japan", "jp")):
        if word in tags:
            pref.append(reg)
    for reg in ("us", "wor", "ss", "eu", "jp"):
        if reg not in pref:
            pref.append(reg)
    return pref


def pick(medias, kind, pref):
    have = [m for m in medias if m.get("type") == kind]
    for reg in pref:
        for m in have:
            if m.get("region") == reg:
                return m
    return have[0] if have else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--creds", required=True)
    ap.add_argument("--games", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--only")
    a = ap.parse_args()
    c = creds(a.creds)
    base = dict(c, output="json")
    out = pathlib.Path(a.out).expanduser()
    for line in pathlib.Path(a.games).read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        sysdir, ssid, rom, *note = line.split("\t")
        if a.only and sysdir not in a.only.split(","):
            continue
        stem = rom.rsplit(".", 1)[0]
        gdir = out / sysdir / "raw" / stem
        gdir.mkdir(parents=True, exist_ok=True)
        meta_path = gdir / "meta.json"
        if meta_path.exists():
            meta = json.loads(meta_path.read_text())
        else:
            q = dict(base, systemeid=ssid, romtype="rom", romnom=rom)
            code, body = get(API + "jeuInfos.php?" + urllib.parse.urlencode(q))
            jeu = None
            if code == 200:
                try:
                    jeu = json.loads(body)["response"]["jeu"]
                except Exception:
                    jeu = None
            how = "romnom"
            if not jeu:
                term = re.sub(r"\s*\([^)]*\)", "", stem).strip()
                if ", The" in term:
                    term = term.replace(", The", "")
                q = dict(base, systemeid=ssid, recherche=term)
                code, body = get(API + "jeuRecherche.php?" + urllib.parse.urlencode(q))
                how = "recherche"
                try:
                    jeux = json.loads(body)["response"]["jeux"]
                    jeu = jeux[0] if jeux and jeux[0] else None
                except Exception:
                    jeu = None
            if not jeu:
                print(f"{sysdir}\t{stem}\tNOT FOUND (http {code})", flush=True)
                meta = {"found": False}
                meta_path.write_text(json.dumps(meta))
                continue
            names = jeu.get("noms") or []
            title = next((n["text"] for n in names if n.get("region") in ("ss", "us", "wor")), names[0]["text"] if names else stem)
            pref = regions_for(rom)
            medias = jeu.get("medias") or []
            meta = {"found": True, "how": how, "title": title, "jeuid": jeu.get("id"), "region_pref": pref,
                    "available": sorted({m.get("type") for m in medias if m.get("type")}), "media": {}}
            for kind in TYPES:
                m = pick(medias, kind, pref)
                if m:
                    meta["media"][kind] = {"region": m.get("region"), "format": m.get("format"), "_url": m["url"]}
        # download (urls only live in memory/this run; never persisted)
        for kind, m in meta.get("media", {}).items():
            dest = gdir / f"{kind}.{m.get('format') or 'png'}"
            if dest.exists() and dest.stat().st_size > 0:
                continue
            url = m.get("_url")
            if not url:
                continue
            if kind not in ("wheel", "wheel-hd"):
                url += f"&maxwidth={MAXW}&maxheight={MAXW}"
            code, body = get(url)
            if code == 200 and len(body) > 200 and not body[:5].lower().startswith((b"<html", b"{", b"no")):
                dest.write_bytes(body)
                m["bytes"] = len(body)
            else:
                m["error"] = f"http {code}: {redact(body[:80])}"
        saved = {"found": meta.get("found")}
        saved.update({k: v for k, v in meta.items() if k != "media"})
        saved["media"] = {k: {kk: vv for kk, vv in v.items() if kk != "_url"} for k, v in meta.get("media", {}).items()}
        meta_path.write_text(json.dumps(saved, indent=1))
        print(f"{sysdir}\t{stem}\t{saved.get('how')}\t{saved.get('title')}\t" + ",".join(f"{k}:{v.get('region')}" for k, v in saved["media"].items()), flush=True)


if __name__ == "__main__":
    sys.exit(main())
