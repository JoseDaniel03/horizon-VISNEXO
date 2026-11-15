//! Native Android host for the same engine dependency, version, saves and protocol as PC.
//! No alternate game tree or launcher implementation is compiled here.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub mod android_resources;
#[cfg(target_os = "android")]
mod native;
pub mod official_install;
pub mod resource_install;

#[cfg(target_os = "android")]
mod health;
