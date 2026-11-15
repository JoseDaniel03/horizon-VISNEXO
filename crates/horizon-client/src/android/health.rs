//! Safe policy bridge. JNI stays in the Android host, outside the engine.
use horizon_client_render::android_policy::{Governor, Profile};
use std::{
    sync::{LazyLock, Mutex, OnceLock},
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct Reading {
    pub memory_bytes: u64,
    pub low_ram: bool,
    pub thermal: Option<i32>,
    pub trim: i32,
}
static READER: OnceLock<fn() -> Reading> = OnceLock::new();
pub fn initialize(reader: fn() -> Reading) {
    let _ = READER.set(reader);
}
fn reading() -> Reading {
    READER.get().map_or_else(Reading::default, |f| f())
}
#[derive(Default)]
struct State {
    governor: Governor,
    last: Option<Instant>,
    trim: i32,
}
static STATE: LazyLock<Mutex<State>> = LazyLock::new(Mutex::default);
pub fn profile() -> Profile {
    let r = reading();
    Profile::for_memory(r.memory_bytes, r.low_ram)
}
/// Called by the existing frame loop at most every two seconds. No extra thread.
pub fn sample() -> bool {
    let now = Instant::now();
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if state
        .last
        .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_secs(2))
    {
        return false;
    }
    state.last = Some(now);
    let r = reading();
    state.governor.observe(r.thermal, now);
    let should_trim = r.trim >= 10 && r.trim != state.trim;
    state.trim = r.trim;
    should_trim
}
pub fn frame_cap(requested: i32) -> i32 {
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .governor
        .cap(requested)
}
