//! Gameplay and startup, above the reusable client library.
// No `#![recursion_limit]` or other inner attribute horizon-mixins doesn't know here: the
// launcher's horizon-mixins then rejects every native mod targeting this crate (#8169).
#![forbid(unsafe_code)]
extern crate self as horizon_client;
pub use horizon_client_lib::*;
pub mod entry;
pub mod game;
mod horizon_look;
pub mod mod_api;

#[cfg(test)]
mod brewing_speed_load_tests;
#[cfg(test)]
mod living_death_tests;

#[cfg(target_os = "android")]
pub mod android;

/// Platform launch arguments; Android supplies them through the validated native host.
pub fn launch_arguments() -> Vec<String> {
    #[cfg(target_os = "android")]
    { android::arguments() }
    #[cfg(not(target_os = "android"))]
    { std::env::args().skip(1).collect() }
}
