#!/usr/bin/env python3
"""VIS Nexo native Horizon game packages, NO Horizon Launcher or Mojang assets.

Runs on CI for the user's Horizon fork. Never includes the proprietary
client.jar, server.jar, downloaded assets, worlds, or prepared .local files.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent
EXPORTERS = Path("mods/horizon/src/test/java/dev/horizonlab/parity")
BINARIES = ("horizon-client", "horizon-prepare")
FORK_SOURCE = "https://github.com/JoseDaniel03/horizon-VISNEXO.git"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def platform_id() -> str:
    machine = platform.machine().lower()
    if machine not in {"amd64", "x86_64"}:
        raise RuntimeError("VIS Nexo portable supports only x86-64 for now")
    if os.name == "nt":
        return "windows-x64"
    if platform.system() == "Linux":
        return "linux-x86_64"
    raise RuntimeError("Unsupported release platform")


def binary(name: str) -> str:
    return name + (".exe" if os.name == "nt" else "")


def stage_game(source: Path, commit: str, destination: Path) -> Path:
    if len(commit) != 40 or any(c not in "0123456789abcdef" for c in commit.lower()):
        raise ValueError("Full 40-digit source commit is required")
    if destination.exists():
        raise ValueError("Staging directory already exists")
    destination.mkdir(parents=True)
    for name in BINARIES:
        executable = source / "target" / "release" / binary(name)
        if not executable.is_file():
            raise FileNotFoundError(f"Compiled game executable is missing: {executable}")
        shutil.copy2(executable, destination / binary(name))
        if os.name != "nt":
            (destination / binary(name)).chmod(0o755)
    for name in ("research-lock.json", "LICENSE", "THIRD_PARTY_NOTICES.md"):
        src = source / name
        if not src.is_file():
            raise FileNotFoundError(f"Required Horizon metadata missing: {name}")
        shutil.copy2(src, destination / name)
    origin = source / EXPORTERS
    if not origin.is_dir() or not (origin / "VanillaHeightmapOracle.java").is_file():
        raise FileNotFoundError("Horizon preparation exporter sources are missing")
    shutil.copytree(
        origin, destination / EXPORTERS, symlinks=False,
        ignore=shutil.ignore_patterns("*.class", "*.jar", "*.zip", "*.log", "__pycache__")
    )
    hashes = {
        path.relative_to(destination).as_posix(): sha256(path)
        for path in destination.rglob("*") if path.is_file()
    }
    meta = {
        "format": 1, "source_repo": FORK_SOURCE,
        "upstream_repo": "https://github.com/horizon-rust/horizon.git",
        "source_commit": commit, "source_ref": "main",
        "platform": platform_id(), "sha256": hashes,
        "minecraft_assets_included": False,
        "note": "Horizon Rust game only; player obtains Mojang files separately."
    }
    (destination / "vis-horizon-source.json").write_text(
        json.dumps(meta, sort_keys=True, indent=2) + "\n", encoding="utf-8"
    )
    return destination


def make_archive(stage: Path, dist: Path) -> Path:
    dist.mkdir(parents=True, exist_ok=True)
    if platform_id().startswith("windows"):
        archive = dist / "vis-horizon-windows-x64.zip"
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as output:
            for path in sorted(stage.rglob("*")):
                if path.is_file():
                    output.write(path, "vis-horizon/" + path.relative_to(stage).as_posix())
    else:
        archive = dist / "vis-horizon-linux-x86_64.tar.gz"
        with tarfile.open(archive, "w:gz", compresslevel=9) as output:
            for path in sorted(stage.rglob("*")):
                if path.is_file():
                    info = output.gettarinfo(str(path), arcname="vis-horizon/" + path.relative_to(stage).as_posix())
                    if not info.isfile() or info.issym() or info.islnk():
                        raise RuntimeError("Only regular files belong in a portable game")
                    with path.open("rb") as handle:
                        output.addfile(info, handle)
    (dist / (archive.name + ".sha256")).write_text(
        sha256(archive) + "  " + archive.name + "\n", encoding="ascii"
    )
    print(f"VIS game package: {archive} ({archive.stat().st_size:,} bytes)")
    return archive


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--commit", required=True, help="Exact checked-out git commit")
    parser.add_argument("--dist", default="dist/vis-game")
    args = parser.parse_args()
    dist = (ROOT / args.dist).resolve()
    with tempfile.TemporaryDirectory(prefix="vis-horizon-package-") as tmp:
        stage = stage_game(ROOT, args.commit.lower(), Path(tmp) / "vis-horizon")
        make_archive(stage, dist)


if __name__ == "__main__":
    main()
