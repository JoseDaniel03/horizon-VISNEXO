# Shared PC / Android engine

The Android package is a NativeActivity host for the same `horizon-client` crate and
`horizon-official::register()` hooks used by the desktop executable. Android-specific
code adapts activity/event-loop ownership, touch, private paths and resource installation.
It must not check out a separate gameplay repository.

`vis-mcrust-release.json` is published only after Windows x64, Linux x64 and Android
ARM64 jobs at the same source SHA succeed. Its engine commit, protocol, world data
version and resource/data pack formats are checked by Nexo. The Android handoff also
includes that exact engine commit; the game rejects a different revision.

The launcher downloads the APK itself. Android requires permission to install packages
from Nexo and its normal installation confirmation. After confirming, select Jugar again.
The game has no separate launcher shortcut. Original resources download from official
sources on first game startup. Java loaders are not supported by this Android host.

## Release prerequisites

Configure these repository Actions secrets using a stable signing identity:

- `VIS_MCRUST_ANDROID_KEYSTORE_B64`
- `VIS_MCRUST_ANDROID_STORE_PASSWORD`
- `VIS_MCRUST_ANDROID_KEY_PASSWORD`
- `VIS_MCRUST_ANDROID_KEY_ALIAS`

A missing stable signing identity fails publication. Never silently generate a public
release key, publish a different Android game tree, or fall back to the obsolete preview.
The older `com.visstudios.mcrust.preview` application is not imported or removed.

## Required device acceptance checks

1. Install Nexo on an ARM64 Android device (game minimum API 28). Create a Horizon
   26.3 Vanilla instance and select Jugar. Check permission denial/cancellation,
   installation confirmation and subsequent direct launches. Reject an APK signed
   with an unexpected certificate and a corrupt cached download.
2. Confirm every platform comes from the same full engine SHA. Create a disposable
   PC server/world; join from PC and Android together. Exercise inventory, chunk
   loading, entities, dimensions and resource pack negotiation.
3. Create/save/reopen a disposable Android world. Background/resume while loading
   and playing; check surface recreation, no held touch keys and singleplayer pause.
   Published/online worlds retain the shared engine's multiplayer pause policy.
4. Switch launcher accounts/instances and verify save locations and identity. Check
   repeated Jugar while a game is running and OS process recreation.
5. Test microphone, social overlay and invitations separately before claiming mobile
   parity: compiling shared code alone is not proof of device behavior.

No real player worlds, accounts, prepared Mojang resources or signing keys belong in
build artifacts or this repository. A shared code revision reduces incompatibilities;
it does not establish that every device or feature works without acceptance testing.
