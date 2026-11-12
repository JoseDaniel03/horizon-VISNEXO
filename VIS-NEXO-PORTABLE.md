# Horizon game integration for VIS Nexo

This fork publishes game-only binaries for VIS Nexo. The upstream Horizon project is https://github.com/horizon-rust/horizon and remains the primary MIT-licensed source of the game.

## Build

GitHub Actions workflow: .github/workflows/vis-nexo-portable.yml

Builds exactly these two Rust binaries:
- horizon-client / horizon-client.exe
- horizon-prepare / horizon-prepare.exe

Also includes research-lock.json, LICENSE, THIRD_PARTY_NOTICES.md and Java exporter sources required by resource preparation. It does **not** include any proprietary Minecraft files, resources, prepared worlds or Horizon Launcher.

Outputs:
- vis-horizon-linux-x86_64.tar.gz (Linux desktop)
- vis-horizon-windows-x64.zip (Windows desktop)
- SHA256SUMS (one checksum per archive)

The release name is visgame- plus the first 12 hex characters of the exact source commit. It is immutable: an existing release tag is not overwritten.

## How to publish

1. Enable GitHub Actions for this fork if GitHub has disabled them.
2. Open Actions -> VIS Nexo - Portable Horizon Game -> Run workflow on main.
3. Both platform jobs compile, verify the Horizon Launch API, stage portable archives and upload job artifacts.
4. Once both succeed, the publish job creates a GitHub Release containing both archives and checksums.
5. VIS Nexo can then fetch the appropriate archive when a user installs a Horizon instance. Until then, on-demand installation will explicitly say no release is available.

The Release job needs repository permission to write Releases. GitHub Action permission is restricted to contents:write on that publish job; the native build jobs only read source.

## Integrity boundaries

The VIS Nexo launcher verifies GitHub's SHA-256 asset digest, expected repository and release tag, full source commit in vis-horizon-source.json, package file hashes and archive layout. The build does **not** currently have an independently signed release key. Do not claim it is a signed upstream Horizon Nightly.

The license notices and copyright credits must remain intact. Minecraft Java Edition 26.3 files are downloaded for the owning account from official Mojang endpoints by VIS Nexo on the player's machine. The game resources are never published in the fork.

Do not turn this fork into another required desktop launcher: VIS Nexo is the only user-facing launcher.
