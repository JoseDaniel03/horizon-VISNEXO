//! Horizon: native Rust client for Minecraft Java Edition 26.3 gameplay.
#![forbid(unsafe_code)]

#[cfg(not(any(feature = "system-allocator", feature = "alloc-census")))]
#[global_allocator]
static ALLOCATOR: horizon_threads::Allocator = horizon_threads::ALLOCATOR;
#[cfg(all(feature = "alloc-census", not(feature = "system-allocator")))]
#[global_allocator]
static ALLOCATOR: horizon_threads::CensusAllocator = horizon_threads::CensusAllocator;

/// The mod API level this game serves, kept in the program for the launcher
/// to read without running it (`horizon_mods::api_level::MARK`). `main`
/// reads it, so the linker keeps it.
static MOD_API_LEVEL: [u8; 27] = horizon_mods::api_level::MARK;

// The code below is ONLY included in VIS builds. A .hnmod placed by a player
// never gets executed from disk: the user-selected marker activates exactly
// the VSClient source reviewed and compiled with this binary.
#[cfg(feature = "vis-vsclient")]
#[path = "../../../integrations/vsclient-horizon/api/src/lib.rs"]
mod vis_vsclient;

#[cfg(feature = "vis-vsclient")]
#[path = "../../../integrations/vsclient-horizon/activation.rs"]
mod vis_autoload;

fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    #[cfg(feature = "vis-vsclient")]
    if vis_autoload::wants_vsclient() {
        horizon_client::mod_api::name_mods(&["vsclient"]);
        let mods: horizon_client::mod_api::Mods =
            std::sync::Arc::from(vec![vis_vsclient::hooks()]);
        horizon_client::entry::main_with(mods);
        return;
    }
    horizon_client::entry::main();
}

