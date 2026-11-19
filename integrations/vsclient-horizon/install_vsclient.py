#!/usr/bin/env python3
"""Install the VSClient Horizon .hnmod into ONE selected game instance.

Does not compile, execute, or trust native Rust. The native build must be
started from Horizon's own UI, with the player's explicit trust approval.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import tempfile
import zipfile
from pathlib import Path

ID = "vsclient"
MAX_BYTES = 64 * 1024 * 1024


def validate(path: Path) -> dict:
    if path.suffix.lower() != ".hnmod":
        raise ValueError("Extract the .hnmod from the outer GitHub Actions ZIP first")
    if not path.is_file() or path.is_symlink():
        raise ValueError(f"Native mod file is missing or unsafe: {path}")
    if path.stat().st_size > MAX_BYTES:
        raise ValueError("Native mod exceeds Horizon's 64 MiB download budget")
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)) or len(names) > 512:
            raise ValueError("Duplicate or excessive files in the native mod")
        if "horizon-native.json" not in names or "mixins/vsclient/horizon-mixin.toml" not in names:
            raise ValueError("This is not a Horizon native VSClient mod")
        total = 0
        for entry in archive.infolist():
            if entry.is_dir():
                continue
            name = entry.filename
            parts = name.split("/")
            if (name.startswith("/") or "\\" in name or ":" in name
                    or any(part in ("", ".", "..") for part in parts)):
                raise ValueError(f"Unsafe archive entry: {name}")
            mode = (entry.external_attr >> 16) & 0o170000
            if mode not in (0, 0o100000):
                raise ValueError(f"Archive includes a symbolic link/device: {name}")
            total += entry.file_size
            if total > MAX_BYTES:
                raise ValueError("Expanded archive exceeds Horizon's 64 MiB limit")
        manifest = json.loads(archive.read("horizon-native.json"))
        rev = manifest.get("horizon", "")
        if (manifest.get("schema") != 1 or manifest.get("id") != ID
                or manifest.get("game") != "26.3"
                or not isinstance(manifest.get("version"), str)
                or not isinstance(rev, str) or len(rev) != 40
                or any(c not in "0123456789abcdef" for c in rev)):
            raise ValueError("VSClient manifest is incompatible or invalid")
        if manifest.get("hooks") is not None:
            raise ValueError("Expected fork-pinned VSClient, not the generic 0.4.0 build")
        return manifest


def install(instance: Path, mod: Path, replace: bool = False, dry_run: bool = False) -> Path:
    manifest = validate(mod)
    if not instance.is_dir() or instance.is_symlink():
        raise ValueError(f"Choose an existing Horizon game instance folder: {instance}")
    target_dir = instance / "horizon-native-mods"
    installed = target_dir / "vsclient.hnmod"
    if target_dir.is_symlink() or installed.is_symlink():
        raise ValueError("Refusing symbolic links in the instance native mod folder")
    if installed.exists() and not replace:
        raise FileExistsError(f"{installed} already exists; use --replace to upgrade")
    if dry_run:
        print(f"Valid VSClient {manifest['version']}; would install at {installed}")
        return installed
    target_dir.mkdir(parents=True, exist_ok=True)
    fd, filename = tempfile.mkstemp(prefix=".vsclient-", suffix=".hnmod", dir=target_dir)
    try:
        with os.fdopen(fd, "wb") as out, mod.open("rb") as inp:
            shutil.copyfileobj(inp, out, 1024 * 1024)
            out.flush()
            os.fsync(out.fileno())
        os.replace(filename, installed)
    finally:
        if os.path.exists(filename):
            os.unlink(filename)
    # Horizon's installed_mods_into intentionally ignores a copied .hnmod
    # unless "mod.vsclient:true" is enabled in horizon-mods.txt.
    prefs = instance / "horizon-mods.txt"
    if prefs.is_symlink():
        raise ValueError("Refusing a symbolic-link mod settings file")
    lines = prefs.read_text(encoding="utf-8").splitlines() if prefs.exists() else []
    lines = [line for line in lines
             if line.partition(":")[0].strip() != "mod.vsclient"]
    lines.append("mod.vsclient:true")
    fd, config = tempfile.mkstemp(prefix=".horizon-mods-", suffix=".txt", dir=instance)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as out:
            out.write("\n".join(lines) + "\n")
            out.flush()
            os.fsync(out.fileno())
        os.replace(config, prefs)
    finally:
        if os.path.exists(config):
            os.unlink(config)
    print(f"Registered VSClient {manifest['version']} in {instance}")
    print("Native code is NOT running yet: open Horizon's Native build screen")
    print("and explicitly approve 'I trust these mods: build'.")
    return installed


def main() -> None:
    p = argparse.ArgumentParser(description="Install VSClient for Horizon/MCRust desktop")
    p.add_argument("--instance", required=True, type=Path, help="Game instance root, not mods/")
    p.add_argument("--hnmod", required=True, type=Path, help="Extracted VSClient .hnmod")
    p.add_argument("--replace", action="store_true", help="Upgrade a previous VSClient .hnmod")
    p.add_argument("--check-only", action="store_true", help="Validate without writing anything")
    args = p.parse_args()
    try:
        install(args.instance, args.hnmod, args.replace, args.check_only)
    except (OSError, ValueError, zipfile.BadZipFile) as e:
        p.exit(1, f"VSClient native installation failed: {e}\n")


if __name__ == "__main__":
    main()
