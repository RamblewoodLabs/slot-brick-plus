#!/usr/bin/env python3
"""Builds the offline shelf-filter rankings in crates/slot-store/rankings/<SYS>.tsv.

Sources (downloaded once into --src, not committed):
  launchbox/Metadata.xml, launchbox/Mame.xml
      LaunchBox Games Database public export, https://gamesdb.launchbox-app.com/Metadata.zip
      CommunityRating / CommunityRatingCount (user scores and how many rated), Publisher.
  lrdb/metadat/publisher/*.dat
      libretro-database (https://github.com/libretro/libretro-database), publisher by No-Intro name.
  wiki/<SYS>.html
      Wikipedia "List of best-selling ... games" pages (REST HTML), CC BY-SA.

Each row: key, top (rank by weighted user score), pop (rank by number of ratings),
sales (rank on Wikipedia's best-selling list), first (1 when the platform holder published it).
0 means no data. The key is rank_key(), which slot-store's rankings.rs repeats exactly.

Usage: make-rankings.py --src DIR [--romlist romlist.tsv] [--coverage out.md]
"""
import argparse, html, json, os, re, statistics, sys, time
import xml.etree.ElementTree as ET
from html.parser import HTMLParser

LB_PLATFORM = {
    "GBA": "Nintendo Game Boy Advance", "GB": "Nintendo Game Boy", "GBC": "Nintendo Game Boy Color",
    "NES": "Nintendo Entertainment System", "SNES": "Super Nintendo Entertainment System",
    "PS1": "Sony Playstation", "MD": "Sega Genesis", "SMS": "Sega Master System", "GG": "Sega Game Gear",
    "ARCADE": "Arcade", "PCE": "NEC TurboGrafx-16", "A2600": "Atari 2600", "INTV": "Mattel Intellivision",
    "LYNX": "Atari Lynx", "A7800": "Atari 7800", "GW": "Nintendo Game & Watch",
}
LR_DAT = {
    "GBA": "Nintendo - Game Boy Advance", "GB": "Nintendo - Game Boy", "GBC": "Nintendo - Game Boy Color",
    "NES": "Nintendo - Nintendo Entertainment System", "SNES": "Nintendo - Super Nintendo Entertainment System",
    "PS1": "Sony - PlayStation", "MD": "Sega - Mega Drive - Genesis", "SMS": "Sega - Master System - Mark III",
    "GG": "Sega - Game Gear", "PCE": "NEC - PC Engine - TurboGrafx 16", "A2600": "Atari - 2600",
    "INTV": "Mattel - Intellivision", "LYNX": "Atari - Lynx", "A7800": "Atari - 7800",
}
# Who made the system: their games are FIRST PARTY. Arcade has no platform holder.
HOLDER = {
    "GBA": r"\bnintendo\b", "GB": r"\bnintendo\b", "GBC": r"\bnintendo\b", "NES": r"\bnintendo\b",
    "SNES": r"\bnintendo\b", "GW": r"\bnintendo\b", "MD": r"\bsega\b", "SMS": r"\bsega\b", "GG": r"\bsega\b",
    "PS1": r"\bsony\b|\bscea\b|\bscee\b|\bscei\b", "PCE": r"\bnec\b|\bhudson\b",
    "A2600": r"\batari\b", "A7800": r"\batari\b", "LYNX": r"\batari\b", "INTV": r"\bmattel\b|\bintv corp",
}
# Wikipedia's best-selling lists; Game Boy's covers the Game Boy Color too.
WIKI = {"NES": "NES", "SNES": "SNES", "GB": "GB", "GBC": "GB", "GBA": "GBA", "PS1": "PS1", "MD": "MD", "A2600": "A2600"}
MIN_RATINGS = 3
PRIOR = 15  # ratings of weight the platform mean gets in the weighted score

ACCENTS = str.maketrans({c: r for r, cs in {
    "a": "àáâãäå", "e": "èéêë", "i": "ìíîï", "o": "òóôõöø", "u": "ùúûü", "n": "ñ", "c": "ç", "y": "ýÿ"}.items() for c in cs})
ROMAN = {"ii": "2", "iii": "3", "iv": "4", "vi": "6", "vii": "7", "viii": "8"}


def rank_key(name):
    """The game's name with its tags, case, accents, punctuation and 'the' taken off."""
    base = re.split(r"\s*[\(\[]", name, maxsplit=1)[0]
    base = base.lower().translate(ACCENTS).replace("&", " and ").replace("'", "")
    words = [w for w in re.split(r"[^a-z0-9]+", base) if w and w != "the"]
    # "Pokemon - Ruby Version" and "Pokemon Ruby" are the same game.
    if len(words) > 1 and words[-1] == "version":
        words.pop()
    return "".join(ROMAN.get(w, w) for w in words)


def load_launchbox(src, cache):
    if os.path.exists(cache):
        return json.load(open(cache))
    want = {v: k for k, v in LB_PLATFORM.items()}
    games, alts = {}, {}
    it = ET.iterparse(os.path.join(src, "launchbox", "Metadata.xml"), events=("start", "end"))
    _, root = next(it)
    root_clear = root.clear
    for ev, el in it:
        if ev != "end":
            continue
        if el.tag == "Game":
            plat = el.findtext("Platform")
            if plat in want:
                games[el.findtext("DatabaseID")] = {
                    "sys": want[plat], "name": el.findtext("Name") or "",
                    "rating": float(el.findtext("CommunityRating") or 0),
                    "count": int(el.findtext("CommunityRatingCount") or 0),
                    "publisher": el.findtext("Publisher") or "",
                }
            el.clear()
            root_clear()
        elif el.tag == "GameAlternateName":
            alts.setdefault(el.findtext("DatabaseID"), []).append(el.findtext("AlternateName") or "")
            root_clear()
        elif el.tag == "GameImage":
            root_clear()
    for gid, g in games.items():
        g["alts"] = alts.get(gid, [])
    mame = {}
    for _, el in ET.iterparse(os.path.join(src, "launchbox", "Mame.xml"), events=("end",)):
        if el.tag == "MameFile":
            mame[el.findtext("FileName")] = el.findtext("Name") or ""
            el.clear()
    data = {"games": list(games.values()), "mame": mame}
    json.dump(data, open(cache, "w"))
    return data


def load_publishers(src, sys_):
    name = LR_DAT.get(sys_)
    out = {}
    if not name:
        return out
    path = os.path.join(src, "lrdb", "metadat", "publisher", name + ".dat")
    if not os.path.exists(path):
        return out
    text = open(path, encoding="utf-8", errors="replace").read()
    for m in re.finditer(r'game \(\s*(?:comment|name) "([^"]*)"\s*publisher "([^"]*)"', text):
        out.setdefault(rank_key(m.group(1)), set()).add(m.group(2))
    return out


class Tables(HTMLParser):
    """Every wikitable as rows of cell text, rowspans filled in."""

    def __init__(self):
        super().__init__()
        self.tables, self.depth, self.cell, self.row, self.spans = [], 0, None, None, {}
        self.skip = 0

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "table":
            self.depth += 1
            if self.depth == 1 and "wikitable" in (a.get("class") or ""):
                self.tables.append([])
                self.spans = {}
            return
        if not self.tables or self.depth != 1:
            return
        if tag == "tr":
            self.row = []
        elif tag in ("td", "th") and self.row is not None:
            self.cell = {"text": "", "rs": int(re.sub(r"\D", "", a.get("rowspan") or "1") or 1),
                         "cs": int(re.sub(r"\D", "", a.get("colspan") or "1") or 1)}
        elif tag in ("sup", "style") and self.cell is not None:
            self.skip += 1
        elif tag == "br" and self.cell is not None:
            self.cell["text"] += " "

    def handle_endtag(self, tag):
        if tag == "table":
            self.depth -= 1
            return
        if not self.tables or self.depth != 1:
            return
        if tag in ("sup", "style") and self.skip:
            self.skip -= 1
        elif tag in ("td", "th") and self.cell is not None and self.row is not None:
            self.row.append(self.cell)
            self.cell = None
        elif tag == "tr" and self.row is not None:
            cells, col, it = [], 0, iter(self.row)
            pending = list(self.row)
            while pending or col in self.spans:
                if col in self.spans:
                    text, left = self.spans[col]
                    cells.append(text)
                    if left <= 1:
                        del self.spans[col]
                    else:
                        self.spans[col] = (text, left - 1)
                    col += 1
                    continue
                c = pending.pop(0)
                t = re.sub(r"\s+", " ", c["text"]).strip()
                for _ in range(c["cs"]):
                    if c["rs"] > 1:
                        self.spans[col] = (t, c["rs"] - 1)
                    cells.append(t)
                    col += 1
            self.tables[-1].append(cells)
            self.row = None

    def handle_data(self, data):
        if self.cell is not None and not self.skip:
            self.cell["text"] += data


def sales_number(text):
    t = text.lower().replace(",", "")
    m = re.search(r"(\d+(?:\.\d+)?)\s*(million|m\b)?", t)
    if not m:
        return None
    v = float(m.group(1))
    return v * 1_000_000 if m.group(2) else v


def sales_keys(title):
    """Wikipedia lists pairs as one row ("Pokemon Gold and Silver", "Sonic 3 / Sonic & Knuckles")
    and puts subtitles either side of a colon: each name it could be is a key."""
    keys = [rank_key(title)]
    for sep in (r"\s+and\s+|,\s*", r"\s*/\s*"):
        parts = [p for p in re.split(sep, title) if p.strip()]
        if len(parts) > 1:
            stem = parts[0].split()[:-1]
            keys += [rank_key(parts[0])] + [rank_key(" ".join(stem + [p]) if len(p.split()) == 1 else p) for p in parts[1:]]
    for part in re.split(r"\s*:\s*", title):
        if len(part.split()) >= 2:
            keys.append(rank_key(part))
    return list(dict.fromkeys(k for k in keys if k))


def load_sales(src, page):
    path = os.path.join(src, "wiki", page + ".html")
    if not os.path.exists(path):
        return []
    p = Tables()
    p.feed(open(path, encoding="utf-8").read())
    best = {}
    for table in p.tables:
        if not table:
            continue
        head = [h.lower() for h in table[0]]
        gi = next((i for i, h in enumerate(head) if h.startswith(("game", "title"))), None)
        si = next((i for i, h in enumerate(head) if "sales" in h or "sold" in h), None)
        pi = next((i for i, h in enumerate(head) if h.startswith("platform")), None)
        if gi is None or si is None:
            continue
        for row in table[1:]:
            if len(row) <= max(gi, si):
                continue
            n = sales_number(row[si])
            title = re.sub(r"\[.*?\]", "", row[gi]).strip()
            if not n or not title:
                continue
            plat = row[pi] if pi is not None and pi < len(row) else ""
            for k in sales_keys(title):
                if k and n > best.get(k, (0, ""))[0]:
                    best[k] = (n, plat)
    return sorted(best.items(), key=lambda kv: -kv[1][0])


def build(src, cache):
    lb = load_launchbox(src, cache)
    out = {}
    for sys_ in LB_PLATFORM:
        games = [g for g in lb["games"] if g["sys"] == sys_]
        rated = [g for g in games if g["count"] >= MIN_RATINGS and g["rating"] > 0]
        mean = statistics.mean(g["rating"] for g in rated) if rated else 0
        for g in rated:
            v = g["count"]
            g["score"] = (v * g["rating"] + PRIOR * mean) / (v + PRIOR)
        top = {id(g): i + 1 for i, g in enumerate(sorted(rated, key=lambda g: (-g["score"], -g["count"], g["name"])))}
        pop = {id(g): i + 1 for i, g in enumerate(sorted(rated, key=lambda g: (-g["count"], -g["score"], g["name"])))}
        rows = {}
        holder = re.compile(HOLDER[sys_], re.I) if sys_ in HOLDER else None

        def put(key, t=0, p=0, s=0, f=0):
            if not key:
                return
            r = rows.setdefault(key, [0, 0, 0, 0])
            for i, v in enumerate((t, p, s)):
                if v and (not r[i] or v < r[i]):
                    r[i] = v
            r[3] |= f

        if sys_ == "ARCADE":
            by_name = {}
            for g in games:
                by_name.setdefault(g["name"].lower(), []).append(g)
                for a in g["alts"]:
                    by_name.setdefault(a.lower(), []).append(g)
            for short, name in lb["mame"].items():
                for g in by_name.get(name.lower(), []):
                    put(short.lower(), top.get(id(g), 0), pop.get(id(g), 0))
        else:
            for g in games:
                first = int(bool(holder and holder.search(g["publisher"])))
                for n in [g["name"]] + g["alts"]:
                    put(rank_key(n), top.get(id(g), 0), pop.get(id(g), 0), 0, first)
            if holder:
                for k, pubs in load_publishers(src, sys_).items():
                    if any(holder.search(p) for p in pubs):
                        put(k, f=1)
            if sys_ in WIKI:
                for i, (k, (n, plat)) in enumerate(load_sales(src, WIKI[sys_])):
                    put(k, s=i + 1)
        out[sys_] = {k: r for k, r in rows.items() if any(r)}
    return out


def write(out, dest, stamp):
    os.makedirs(dest, exist_ok=True)
    for sys_, rows in out.items():
        lines = [f"# Slot shelf rankings for {sys_}, made by brick/make-rankings.py ({stamp}).",
                 "# Sources: LaunchBox Games Database (community ratings), libretro-database (publisher),",
                 "# Wikipedia best-selling lists (CC BY-SA). See brick/SOURCES.md.",
                 "# key\ttop\tpop\tsales\tfirst"]
        lines += [f"{k}\t{r[0]}\t{r[1]}\t{r[2]}\t{r[3]}" for k, r in sorted(rows.items())]
        open(os.path.join(dest, sys_ + ".tsv"), "w").write("\n".join(lines) + "\n")


def coverage(out, romlist):
    games = {}
    for line in open(romlist, encoding="utf-8-sig"):
        parts = line.rstrip("\n").split("\t")
        if len(parts) != 2:
            continue
        sys_, fname = parts
        stem, ext = os.path.splitext(fname)
        if sys_ == "PS1" and ext.lower() not in (".chd", ".cue", ".m3u", ".pbp"):
            continue
        if sys_ == "PS1" and "(Track" in stem:
            continue
        k = stem.lower() if sys_ == "ARCADE" else rank_key(stem)
        games.setdefault(sys_, {})[k] = stem
    rows = ["| System | Games | Rated (TOP 100 / POPULAR) | BEST SELLING | FIRST PARTY |", "|---|---|---|---|---|"]
    for sys_ in LB_PLATFORM:
        keys = games.get(sys_, {})
        r = out.get(sys_, {})
        rated = sum(1 for k in keys if r.get(k, [0])[0])
        sales = sum(1 for k in keys if r.get(k, [0, 0, 0])[2])
        first = sum(1 for k in keys if r.get(k, [0, 0, 0, 0])[3])
        n = len(keys)
        pct = lambda x: f"{x} ({100 * x // n if n else 0}%)"
        rows.append(f"| {sys_} | {n} | {pct(rated)} | {pct(sales)} | {pct(first)} |")
    return "\n".join(rows) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", required=True)
    ap.add_argument("--dest", default=os.path.join(os.path.dirname(__file__), "..", "crates", "slot-store", "rankings"))
    ap.add_argument("--romlist")
    ap.add_argument("--coverage")
    a = ap.parse_args()
    out = build(a.src, os.path.join(a.src, "launchbox-subset.json"))
    write(out, a.dest, time.strftime("%Y-%m-%d"))
    if a.romlist:
        text = coverage(out, a.romlist)
        print(text)
        if a.coverage:
            open(a.coverage, "w").write(text)


if __name__ == "__main__":
    main()
