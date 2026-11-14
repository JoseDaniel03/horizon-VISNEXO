# VIS Portals native world sharing

VIS Nexo can share a running singleplayer world and join an invitation without mods or the Horizon launcher.
The official executable (`horizon-official`, packaged as `horizon-client`) is required for the friend tunnel hook.

The launcher sets `VIS_PORTALS_ENDPOINT` to an absolute per-instance JSON path. The game creates a loopback
TCP descriptor with a random token and private file permissions, using the existing local transport. It is
independent of AI control and exposes only `hello`, `host`, `status`, `stop`, and `join friend:CODE`.
Commands execute on the window thread. No cheats, console commands, filesystem operations or arbitrary
server addresses are available on this endpoint. Dedicated and integrated server games do not create it.
Joining requires leaving the current world first; VIS never silently discards a world to accept an invite.

VIS Nexo's encrypted social inbox delivers the code to an authenticated friend. The existing encrypted
`horizon-friends` transport carries the actual game directly or through the player's configured relay.
There is no default public relay. A reachable relay/VPN is still needed when direct networking fails.

Social invitations expire after ten minutes and are accepted once. The launcher renews a world-room lease
every thirty seconds; it expires ninety seconds after the last renewal. This is an invitation delivery lease,
not a new game transport identity check. A code already received remains a bearer credential until sharing
stops or the game changes its code. It must not be treated as per-friend admission or individual eviction.
Minecraft online-mode authentication continues to verify game identities independently.

## Verification status

- Restricted command parser: Rust tests passed.
- VIS Nexo: cargo check passed; dedicated social and UI flow tests passed.
- Full Horizon workspace compilation and two-PC gameplay have not been run in this environment.
- Loopback transport runtime testing is blocked by this environment's socket permissions.
- Service migration 0011 and the new Worker must be deployed together before using invitations.

## Android feasibility

The current fork contains `android_runtime.rs`, `android_gamepad.rs`, and `touch/` input code. This is
reusable input work, not proof of an installable Android game. No Android APK, device test, Android activity
entry point or release pipeline was found in this audit. Remaining work includes Android build/packaging,
window and graphics lifecycle, audio, storage, authentication, touch UI and device performance testing.
The intended mobile client would use Horizon's Java-compatible protocol to join the same worlds as PC;
it would not turn Minecraft Bedrock for Android into a Java client. No Android support is claimed here.
