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
fn game_home(args: impl IntoIterator<Item = std::ffi::OsString>) -> Option<std::path::PathBuf> {
    let mut args=args.into_iter();
    while let Some(arg)=args.next() {
        if arg=="--game-dir" || arg=="--home" {
            let dir=std::path::PathBuf::from(args.next()?);
            return dir.is_dir().then_some(dir);
        }
    }
    None
}

#[cfg(feature = "vis-vsclient")]
fn wants_vsclient() -> bool {
    let Some(home)=game_home(std::env::args_os()) else {return false;};
    let file=home.join("horizon-native-mods/vsclient.hnmod");
    let Ok(metadata)=std::fs::symlink_metadata(&file) else {return false;};
    if !metadata.is_file() || metadata.file_type().is_symlink() {return false;}
    // Respect the native Mods page's choice. VIS Nexo sets this entry when
    // moving a validated mod to its per-instance native mod folder.
    let enabled=std::fs::read_to_string(home.join("horizon-mods.txt"))
        .ok().is_some_and(|s|s.lines().filter_map(|l|l.split_once(':'))
            .any(|(k,v)|k.trim()=="mod.vsclient" && v.trim()=="true"));
    if !enabled {return false;}
    // Strict version match avoids telling users 0.5 or another author's
    // native source is running when the binary only contains 0.4.1.
    match horizon_mods::index::native::Bundle::read(&file) {
        Ok(bundle) if bundle.manifest.id=="vsclient"
            && bundle.manifest.version=="0.4.1"
            && bundle.manifest.game=="26.3" => true,
        Ok(_) | Err(_) => {
            eprintln!("[VIS] VSClient installed but unrecognized or version-incompatible; update the mod/engine");
            false
        }
    }
}

fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    #[cfg(feature = "vis-vsclient")]
    if wants_vsclient() {
        horizon_client::mod_api::name_mods(&["vsclient"]);
        let mods: horizon_client::mod_api::Mods =
            std::sync::Arc::from(vec![vis_vsclient::hooks()]);
        horizon_client::entry::main_with(mods);
        return;
    }
    horizon_client::entry::main();
}

