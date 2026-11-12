#!/usr/bin/env python3
"""Verify the *contents* of a VIS Horizon release archive before publication.

Checks archives without extracting them. This is deliberately independent from
the packaging code so that a packaging regression cannot approve itself.
This does not replace a publisher signature or a real game startup test.
"""
from __future__ import annotations

import argparse
from contextlib import closing
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import stat
import tarfile
import zipfile

SOURCE_REPO = "https://github.com/JoseDaniel03/horizon-VISNEXO.git"
ROOT = "vis-horizon/"
MANIFEST = ROOT + "vis-horizon-source.json"
MAX_FILES = 12_000
MAX_FILE_BYTES = 512 * 1024 * 1024
MAX_TOTAL_BYTES = 1024 * 1024 * 1024


def checked_name(name: str) -> str:
    if not name.startswith(ROOT) or "\\" in name or "\x00" in name:
        raise ValueError(f"Invalid archive prefix/path: {name!r}")
    parts = PurePosixPath(name).parts
    if any(part in ("", ".", "..") or ":" in part for part in name.split("/")):
        raise ValueError(f"Unsafe archive entry: {name!r}")
    if len(parts) < 2 or parts[0] != "vis-horizon":
        raise ValueError(f"Unexpected archive layout: {name!r}")
    if name.lower().endswith((".jar", ".class")):
        raise ValueError(f"Java/Mojang binary is not part of the Horizon engine: {name}")
    return name.removeprefix(ROOT)


def digest_stream(stream) -> str:
    sha = hashlib.sha256()
    for chunk in iter(lambda: stream.read(1024 * 1024), b""):
        sha.update(chunk)
    return sha.hexdigest()


def validate(archive: Path, commit: str, platform: str) -> int:
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("Expected full lowercase 40-character commit")
    if platform not in ("windows-x64", "linux-x86_64"):
        raise ValueError("Unsupported platform")
    expected_archive = ("vis-horizon-windows-x64.zip" if platform == "windows-x64"
                        else "vis-horizon-linux-x86_64.tar.gz")
    if archive.name != expected_archive:
        raise ValueError(f"Archive should be named {expected_archive}")

    if platform == "windows-x64":
        container = zipfile.ZipFile(archive, "r")
        entries = container.infolist()
        def name(entry):
            return entry.filename
        def size(entry):
            return entry.file_size
        def valid(entry):
            return not entry.is_dir() and not stat.S_ISLNK(entry.external_attr >> 16)
        def open_entry(entry):
            return container.open(entry)
    else:
        container = tarfile.open(archive, "r:gz")
        entries = container.getmembers()
        def name(entry):
            return entry.name
        def size(entry):
            return entry.size
        def valid(entry):
            return entry.isfile() and not entry.issym() and not entry.islnk()
        def open_entry(entry):
            result = container.extractfile(entry)
            if result is None:
                raise ValueError(f"Unreadable archive member: {name(entry)}")
            return result

    with closing(container):
        if not 1 <= len(entries) <= MAX_FILES + 1:
            raise ValueError("Archive file count out of bounds")
        found = {}
        total = 0
        for entry in entries:
            if not valid(entry):
                raise ValueError(f"Non-regular archive member: {name(entry)}")
            relative = checked_name(name(entry))
            if relative in found:
                raise ValueError(f"Duplicate archive entry: {relative}")
            if size(entry) < 0 or size(entry) > MAX_FILE_BYTES:
                raise ValueError(f"Archive member too large: {relative}")
            total += size(entry)
            if total > MAX_TOTAL_BYTES:
                raise ValueError("Archive expands beyond 1 GiB")
            found[relative] = entry

        manifest_entry = found.pop("vis-horizon-source.json", None)
        if manifest_entry is None or size(manifest_entry) > 256 * 1024:
            raise ValueError("Missing/oversized source manifest")
        with open_entry(manifest_entry) as stream:
            metadata = json.load(stream)
        if (metadata.get("source_repo") != SOURCE_REPO
                or metadata.get("source_commit") != commit
                or metadata.get("game_binary") != "horizon-official"
                or metadata.get("platform") != platform
                or metadata.get("minecraft_assets_included") is not False):
            raise ValueError("Release provenance, platform, or game identity mismatch")

        hashes = metadata.get("sha256")
        if not isinstance(hashes, dict) or set(hashes) != set(found):
            raise ValueError("Manifest file list does not match archive contents")
        suffix = ".exe" if platform == "windows-x64" else ""
        required = {"horizon-client" + suffix, "horizon-prepare" + suffix,
                    "research-lock.json", "LICENSE", "THIRD_PARTY_NOTICES.md",
                    "mods/horizon/src/test/java/dev/horizonlab/parity/VanillaHeightmapOracle.java"}
        if platform == "windows-x64":
            required.update({"dxcompiler.dll", "DXC-LICENSE-MS.txt",
                             "DXC-LICENSE-LLVM.txt", "DXC-NOTICE.txt"})
        missing = required - set(found)
        if missing:
            raise ValueError(f"Required components missing: {sorted(missing)}")
        # Restrict releases to explicitly required runtime files and open Java
        # source exporters. No client.jar, game assets, worlds or unrelated blobs.
        exporter = "mods/horizon/src/test/java/dev/horizonlab/parity/"
        unexpected = [
            entry for entry in found
            if entry not in required and not (
                entry.startswith(exporter) and entry.endswith(".java")
            )
        ]
        if unexpected:
            raise ValueError(f"Unexpected files in game distribution: {unexpected[:5]}")

        for relative, entry in found.items():
            expected = hashes[relative]
            if not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{64}", expected):
                raise ValueError(f"Invalid SHA-256 manifest entry: {relative}")
            with open_entry(entry) as stream:
                actual = digest_stream(stream)
            if actual != expected:
                raise ValueError(f"Archive file checksum mismatch: {relative}")
        print(f"Validated {platform}: {len(found)} payload files, {total:,} bytes expanded, commit {commit[:12]}")
        return len(found)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--platform", choices=("windows-x64", "linux-x86_64"), required=True)
    args = parser.parse_args()
    validate(args.archive, args.commit.lower(), args.platform)


if __name__ == "__main__":
    main()
