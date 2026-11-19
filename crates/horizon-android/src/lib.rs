//! Native Android host for the same engine dependency, version, saves and protocol as PC.
//! No alternate game tree or launcher implementation is compiled here.
// VSClient's reviewed source is compiled for both desktop and Android.
// The native hook types come from the platform facade on Android; this local
// alias keeps the shared module's `horizon_client::mod_api` imports valid
// without making the Horizon-layer Android host depend on an engine crate.
#[cfg(all(target_os = "android", feature = "vis-vsclient"))]
extern crate horizon_game as horizon_client;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub mod android_resources;
pub mod official_install;
pub mod resource_install;
#[cfg(target_os = "android")]
mod native;
