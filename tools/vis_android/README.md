# Experimental VIS McRust Android port

The desktop engine remains on the existing VIS fork. The Android experiment reuses the native Android port from `starling-client/starling`, pinned to `fa548eb7abe91c194d3780e4580b9746323cd606` under its original MIT license. It is a different reviewed source baseline, not a completed merge of the current desktop engine. No upstream deployment or signing workflow is activated.

The isolated Android package is `com.visstudios.mcrust.preview`. It accepts an explicit activity handoff containing only a validated offline name and server address. On Play, the native port prepares its own resources then enters the real Rust game. No VIS passwords, session tokens, arbitrary paths or CLI flags cross the app boundary. A selected Microsoft account in the game retains its own authentication behavior.

The preview uses a temporary Android signing key generated inside each build. Keys are deleted and never uploaded; subsequent builds can require a reinstall. Back up preview worlds before replacing installations. The public APK payload audit remains enabled. Original Minecraft resources are downloaded by the port, not committed to this repository or attached as prepared-resource archives.

The desktop native Portals overlay, VIS chat identities, world invitations and server voice are not automatically present in this separate Android baseline. The game maps each validated VIS owner/instance ID to its own sandboxed vis-worlds directory; Nexo metadata folders are separate from those actual game saves. This is an experimental game port for device testing, not PC/Android feature parity. APK compilation and physical-device startup must be verified before claiming it works.
