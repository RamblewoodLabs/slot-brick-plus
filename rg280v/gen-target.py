#!/usr/bin/env python3
"""Write a hard-float uClibc Rust target spec for the OpenDingux gcw0 toolchain.

Starts from rustc's built-in mipsel-unknown-linux-uclibc (Tier 3, soft-float)
and patches it so the ABI matches the JZ4770 SDK: uClibc, MIPS32r2, hard float.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

LINKER = "/opt/gcw0-toolchain/bin/mipsel-linux-gcc"
BUILTIN = "mipsel-unknown-linux-uclibc"


def rustc_spec() -> dict:
    out = subprocess.check_output(
        [
            "rustc",
            "-Z",
            "unstable-options",
            "--print",
            "target-spec-json",
            "--target",
            BUILTIN,
        ],
        text=True,
    )
    return json.loads(out)


def harden(spec: dict) -> dict:
    spec.pop("is-builtin", None)
    spec["vendor"] = "gcw0"
    spec["env"] = "uclibc"
    spec["linker"] = LINKER
    feat = str(spec.get("features") or "")
    parts = [p for p in feat.split(",") if p and "soft" not in p.lower()]
    for required in ("+mips32r2", "+fpxx", "+nooddspreg"):
        if required not in parts:
            parts.append(required)
    # FPXX ("Hard float (32-bit CPU, Any FPU)") is the gcw0 gcc 11 default.
    # Plain +mips32r2 hard-float otherwise becomes FP64 ("double precision"),
    # which does not match the toolchain and can crash on a 32-bit FPU.
    spec["features"] = ",".join(parts)
    # The builtin target is O32; keep it. Do not switch to musl or glibc.
    return spec


def main() -> None:
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "mipsel-gcw0-linux-uclibc.json")
    spec = harden(rustc_spec())
    dest.write_text(json.dumps(spec, indent=2) + "\n")
    print(f"wrote {dest}")


if __name__ == "__main__":
    main()
