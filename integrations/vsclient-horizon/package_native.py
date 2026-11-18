#!/usr/bin/env python3
"""Package VSClient as a valid, reproducible Horizon native mod (.hnmod).

Only the mod's Rust and TOML sources are included. The game is compiled by
Horizon when the player explicitly trusts/installs the native mod; this is not
a prebuilt executable, an APK, nor a Fabric JAR.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent.parent
MOD_ID = "vsclient"
GAME = "26.3"
HEX_REV = re.compile(r"[0-9a-f]{40}\Z")
HOOKS = re.compile(r"pub\(crate\) const NATIVE_HOOKS_API: u32 = (\d+);")
MIXINS = re.compile(r'pub const MIXINS_REV: &str = "([0-9a-f]{40})";')


def source_revision(pattern: re.Pattern[str], filename: Path) -> str:
    match = pattern.search(filename.read_text(encoding="utf-8"))
    if match is None:
        raise ValueError(f"Cannot determine native compatibility from {filename}")
    return match.group(1)


def validate_sha(value: str) -> None:
    if not HEX_REV.fullmatch(value):
        raise ValueError("Horizon revision must be an exact 40-character lowercase SHA")


def make_archive(output: Path, horizon_revision: str) -> str:
    validate_sha(horizon_revision)
    hooks = int(source_revision(HOOKS, REPO / "crates/horizon-client/src/game/native_hooks.rs"))
    mixins = source_revision(MIXINS, REPO / "launcher/src/native_mods.rs")
    validate_sha(mixins)
    mixin = ROOT / "horizon-mixin.toml"
    mod = tomllib.loads(mixin.read_text(encoding="utf-8"))
    crate = ROOT / "api/Cargo.toml"
    package = tomllib.loads(crate.read_text(encoding="utf-8"))
    if mod.get("mod", {}).get("id") != MOD_ID:
        raise ValueError("Unexpected native mod ID")
    version = mod["mod"]["version"]
    if version != package["package"]["version"]:
        raise ValueError("Mixin and Cargo versions differ")
    if mod.get("api", {}).get("path") != "api":
        raise ValueError("Horizon native API path must be api")
    if "compatibility" in mod:
        raise ValueError("Source mixin may not pin another Horizon revision")
    manifest = {
        "schema": 1,
        "id": MOD_ID,
        "version": version,
        "game": GAME,
        "horizon": horizon_revision,
        "mixins": mixins,
        "hooks": hooks,
    }
    header = mixin.read_text(encoding="utf-8")
    header += f'\n[compatibility]\nhorizon_revision = "{horizon_revision}"\n'
    sources: dict[str, bytes] = {
        "horizon-native.json": (json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n").encode(),
        f"mixins/{MOD_ID}/horizon-mixin.toml": header.encode(),
    }
    for path in [crate, ROOT / "README.md", *(ROOT / "api/src").rglob("*.rs")]:
        rel = path.relative_to(ROOT).as_posix()
        if not (rel.endswith((".rs", ".toml")) or Path(rel).name.startswith("README")):
            raise ValueError(f"Unsupported source extension: {rel}")
        if not path.is_file() or path.is_symlink():
            raise ValueError(f"Missing source or symbolic link: {path}")
        sources[f"mixins/{MOD_ID}/{rel}"] = path.read_bytes()
    if len(sources) > 512 or sum(len(v) for v in sources.values()) > 64 * 1024 * 1024:
        raise ValueError("Horizon native bundle limit exceeded")
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, data in sorted(sources.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o100644 << 16)
            archive.writestr(info, data, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    with zipfile.ZipFile(output) as archive:
        if archive.testzip() is not None or set(archive.namelist()) != set(sources):
            raise ValueError("Failed to verify .hnmod ZIP contents")
        got = json.loads(archive.read("horizon-native.json"))
        expected = tomllib.loads(archive.read(f"mixins/{MOD_ID}/horizon-mixin.toml").decode())
        if got != manifest or expected["compatibility"]["horizon_revision"] != horizon_revision:
            raise ValueError("Horizon native archive identity/revisions do not match")
    return hashlib.sha256(output.read_bytes()).hexdigest()


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--horizon", required=True, help="Full tested Horizon commit SHA")
    ap.add_argument("--out", required=True, type=Path, help="Output .hnmod")
    args = ap.parse_args()
    if args.out.suffix != ".hnmod":
        ap.error("--out must end in .hnmod")
    sha = make_archive(args.out, args.horizon)
    print(f"{sha}  {args.out}")
