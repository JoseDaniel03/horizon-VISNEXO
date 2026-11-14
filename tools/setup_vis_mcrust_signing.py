#!/usr/bin/env python3
"""Create and upload a stable VIS McRust Android signing identity.

Run ONCE on the maintainer's own computer, not in GitHub Actions or in this
repository. Never prints credentials or writes keys/passwords to Git.
Requires a recent JDK (keytool), GitHub CLI (gh), and an authenticated GitHub
account with Actions Secrets write permission on the fork.
"""
from __future__ import annotations

import base64
import getpass
import os
from pathlib import Path
import shutil
import subprocess
import sys

REPOSITORY = "JoseDaniel03/horizon-VISNEXO"
KEY_DIR = Path.home() / ".config" / "vis-studios" / "mcrust-signing"
KEYSTORE = KEY_DIR / "mcrust-release.p12"
ALIAS = "vis-mcrust"


def fail(message: str) -> None:
    raise SystemExit(message)


def upload(name: str, secret: str) -> None:
    result = subprocess.run(
        ["gh", "secret", "set", name, "--repo", REPOSITORY],
        input=secret,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        # Do not echo the CLI response: it could contain sensitive input on error.
        fail(f"GitHub rejected secret {name}. Verify gh auth and repository permissions.")
    print(f"Stored Actions secret: {name}")


def main() -> None:
    if not shutil.which("gh"):
        fail("Install GitHub CLI (gh) and run gh auth login first.")
    if not shutil.which("keytool"):
        fail("Install a JDK with keytool on PATH first.")
    if not sys.stdin.isatty():
        fail("Run interactively so no signing passwords appear in scripts or logs.")

    auth = subprocess.run(
        ["gh", "auth", "status"], stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL, check=False,
    )
    if auth.returncode:
        fail("GitHub CLI is not authenticated. Run gh auth login.")

    if KEY_DIR.is_symlink():
        fail("Signing directory must not be a symlink.")
    KEY_DIR.mkdir(parents=True, exist_ok=True, mode=0o700)
    if os.name != "nt":
        KEY_DIR.chmod(0o700)
    if KEYSTORE.is_symlink():
        fail("Signing key must not be a symlink.")

    print("VIS McRust release signing — repository:", REPOSITORY)
    print("Keep a protected backup of the keystore and its password.")
    password = getpass.getpass("Signing keystore password (keep in your password manager): ")
    if len(password) < 16:
        fail("Use a password of at least 16 characters.")

    if not KEYSTORE.exists():
        env = os.environ.copy()
        env["VIS_MCRUST_SIGNING_PASSWORD"] = password
        command = [
            "keytool", "-genkeypair", "-v", "-noprompt",
            "-keystore", str(KEYSTORE), "-storetype", "PKCS12",
            "-alias", ALIAS, "-keyalg", "RSA", "-keysize", "3072",
            "-validity", "36500", "-dname", "CN=VIS Studios, O=VIS Studios",
            "-storepass:env", "VIS_MCRUST_SIGNING_PASSWORD",
            "-keypass:env", "VIS_MCRUST_SIGNING_PASSWORD",
        ]
        result = subprocess.run(
            command, env=env, stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL, check=False,
        )
        if result.returncode or not KEYSTORE.is_file():
            fail("Could not create signing keystore. Verify Java keytool support.")
        if os.name != "nt":
            KEYSTORE.chmod(0o600)
        print("Created stable keystore:", KEYSTORE)
    else:
        # A second run must REUSE the original key, never silently rotate it.
        env = os.environ.copy()
        env["VIS_MCRUST_SIGNING_PASSWORD"] = password
        result = subprocess.run(
            ["keytool", "-list", "-keystore", str(KEYSTORE), "-storetype", "PKCS12",
             "-alias", ALIAS, "-storepass:env", "VIS_MCRUST_SIGNING_PASSWORD"],
            env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode:
            fail("Existing keystore/password/alias is invalid; nothing uploaded.")

    try:
        binary = KEYSTORE.read_bytes()
    except OSError:
        fail("Could not read signing keystore.")
    if not binary or len(binary) > 32 * 1024:
        fail("Unexpected signing key length; no secrets uploaded.")

    # GitHub CLI encrypts values for the repository using Actions Secrets API.
    # The keystore does not leave this computer except as an encrypted secret.
    upload("VIS_MCRUST_ANDROID_KEYSTORE_B64", base64.b64encode(binary).decode("ascii"))
    upload("VIS_MCRUST_ANDROID_STORE_PASSWORD", password)
    upload("VIS_MCRUST_ANDROID_KEY_PASSWORD", password)
    upload("VIS_MCRUST_ANDROID_KEY_ALIAS", ALIAS)
    print("Signing identity stored. Keep the original keystore safe for future upgrades.")
    print("Verify the fork build after merging the integration PR to main.")


if __name__ == "__main__":
    main()
