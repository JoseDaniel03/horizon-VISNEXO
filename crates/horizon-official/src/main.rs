//! The official game executable (docs/layers.md, "How a Horizon crate plugs in").
#![forbid(unsafe_code)]

#[global_allocator]
static ALLOCATOR: horizon_game::Allocator = horizon_game::ALLOCATOR;

/// The mod API level this game serves, for the launcher to read without running it; `main`
/// reads it, so the linker keeps it.
static MOD_API_LEVEL: [u8; horizon_game::MOD_API_LEVEL.len()] = horizon_game::MOD_API_LEVEL;

// The reviewed native module shares its source with the Android host.
// Resolve its legacy `horizon_client::mod_api` imports through the
// platform facade, without importing the engine into the Horizon layer.
#[cfg(feature = "vis-vsclient")]
extern crate horizon_game as horizon_client;

#[cfg(feature = "vis-vsclient")]
#[path = "../../../integrations/vsclient-horizon/api/src/lib.rs"]
mod vis_vsclient;
#[cfg(feature = "vis-vsclient")]
#[path = "../../../integrations/vsclient-horizon/activation.rs"]
mod vis_autoload;

fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    horizon_official::register();
    #[cfg(feature = "vis-vsclient")]
    if vis_autoload::wants_vsclient() {
        horizon_game::mod_api::name_mods(&["vsclient"]);
        let mods: horizon_game::mod_api::Mods =
            std::sync::Arc::from(vec![vis_vsclient::hooks()]);
        horizon_game::main_with(mods);
        return;
    }
    horizon_game::main();
}
