//! Server-authoritative VSClient control profiles for Horizon/MCRust.
//! Implements the Java Bridge protocol 8 state/profile/input contract.
//! Pure Rust, no Fabric/GLFW, so protocol tests run without the game.
use std::collections::HashSet;

pub const CHANNEL_STATE: &str = "virgoland:state";
pub const CHANNEL_PROFILE: &str = "virgoland:profile";
pub const CHANNEL_INPUT: &str = "virgoland:input";
pub const CHANNEL_VIEW: &str = "virgoland:view";
const MAX_PROFILE_BYTES: usize = 32_768;
const MAX_ACTIONS: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binding { Key(i32), Mouse(u8), None }
impl Binding {
    fn parse(wire: &str) -> Result<Self, &'static str> {
        let (device, code) = wire.split_once(':').ok_or("Missing binding separator")?;
        let code = code.parse::<i32>().map_err(|_| "Invalid binding code")?;
        match (device, code) {
            ("k", 0..=512) => Ok(Self::Key(code)),
            ("m", 0..=2) => Ok(Self::Mouse(code as u8)),
            ("n", -1) => Ok(Self::None),
            _ => Err("Unsupported control binding"),
        }
    }
    /// Horizon supplies the normal Minecraft key strings for both native
    /// keyboard and mouse input, including gamepad's mapped key events.
    pub fn matches(&self, key: &str) -> bool {
        match self {
            Self::None => false,
            Self::Mouse(n) => match n {
                0 => matches!(key, "key.mouse.left" | "key.mouse.0" | "key.mouse.1"),
                1 => matches!(key, "key.mouse.right" | "key.mouse.2"),
                2 => matches!(key, "key.mouse.middle" | "key.mouse.3"),
                _ => false,
            },
            Self::Key(code) => {
                let Some(name) = key.strip_prefix("key.keyboard.") else { return false };
                if (65..=90).contains(code) {
                    return name.len() == 1 && name.as_bytes()[0] == (*code as u8 + 32);
                }
                if (48..=57).contains(code) {
                    return name.len() == 1 && name.as_bytes()[0] == *code as u8;
                }
                if (290..=301).contains(code) {
                    return name == format!("f{}", code - 289);
                }
                let expected = match code {
                    32 => "space", 256 => "escape", 257 => "enter", 258 => "tab",
                    259 => "backspace", 260 => "insert", 261 => "delete",
                    262 => "right", 263 => "left", 264 => "down", 265 => "up",
                    340 => "left.shift", 341 => "left.control", 342 => "left.alt",
                    344 => "right.shift", 345 => "right.control", 346 => "right.alt",
                    _ => return false,
                };
                name == expected
                    || (expected == "enter" && name == "return")
                    || (expected == "left.shift" && name == "left.shift")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Action {
    pub id: String,
    pub binding: Binding,
    pub phases: u8,
}
#[derive(Clone, Debug)]
pub struct Profile {
    pub revision: u64,
    pub id: String,
    pub state: String,
    pub actions: Vec<Action>,
}
fn valid_state(s: &str) -> bool {
    matches!(s, "OVERWORLD" | "MINIGAME" | "CINEMATIC" | "DRIVING" | "FOOTBALL")
}
fn valid_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 96
        && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()
            || matches!(b, b'_' | b'-' | b'.' | b':' ))
}
pub fn parse_state(wire: &str) -> Result<(u64, String), &'static str> {
    if wire.len() > 160 { return Err("State too long"); }
    let p: Vec<_> = wire.split('|').collect();
    if p.len() != 3 || p[0] != "8" || !valid_state(p[2]) { return Err("Bad state packet"); }
    let rev = p[1].parse().map_err(|_| "Bad state revision")?;
    Ok((rev, p[2].to_owned()))
}
pub fn parse_view(wire: &str) -> Result<f32, &'static str> {
    if wire.len() > 64 { return Err("View too long"); }
    let p: Vec<_> = wire.split('|').collect();
    if p.len() != 2 || p[0] != "8" { return Err("Bad view packet"); }
    let zoom = p[1].parse::<f32>().map_err(|_| "Bad view zoom")?;
    if !zoom.is_finite() || !(1.0..=10.0).contains(&zoom) { return Err("Unsafe zoom"); }
    Ok(zoom)
}
pub fn parse_profile(wire: &str) -> Result<Profile, &'static str> {
    if wire.len() > MAX_PROFILE_BYTES { return Err("Profile too large"); }
    let p: Vec<_> = wire.splitn(7, '|').collect();
    if p.len() < 3 || p[0] != "8" { return Err("Bad profile header"); }
    let revision = p[1].parse::<u64>().map_err(|_| "Bad profile revision")?;
    if p[2] == "-" && p.len() == 3 {
        return Ok(Profile { revision, id: "-".into(), state: "OVERWORLD".into(), actions: vec![] });
    }
    if p.len() != 7 || !valid_id(p[2]) || !valid_state(p[3]) {
        return Err("Bad profile identity");
    }
    // The name fields are URL-safe base64 from the Bridge. We do not display
    // them in this headless version; still reject malformed/unbounded wire.
    let valid_b64 = |s: &str| s.len() <= 256
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'=');
    if !valid_b64(p[4]) { return Err("Bad profile display name"); }
    let count = p[5].parse::<usize>().map_err(|_| "Bad action count")?;
    if count > MAX_ACTIONS { return Err("Too many actions"); }
    let rows: Vec<&str> = if p[6].is_empty() { vec![] } else { p[6].split(';').collect() };
    if rows.len() != count { return Err("Action count mismatch"); }
    let mut seen = HashSet::new();
    let mut actions = Vec::with_capacity(count);
    for row in rows {
        let col: Vec<&str> = row.split(',').collect();
        if col.len() != 4 || !valid_id(col[0]) || !valid_b64(col[1]) || !seen.insert(col[0]) {
            return Err("Bad or duplicate action");
        }
        let phases = col[3].parse::<u8>().map_err(|_| "Bad phases")?;
        if !(1..=3).contains(&phases) { return Err("Unsupported input phases"); }
        actions.push(Action {id: col[0].into(), binding: Binding::parse(col[2])?, phases});
    }
    Ok(Profile {revision, id:p[2].into(), state:p[3].into(), actions})
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputAction { pub id: String, pub pressed: bool, pub sequence: u64 }
#[derive(Default)]
pub struct ControlsSession {
    state: String,
    state_revision: Option<u64>,
    profile: Option<Profile>,
    down: HashSet<String>,
    sequence: u64,
    tick: u64,
    ready: bool,
}
impl ControlsSession {
    pub fn reset(&mut self) { *self = Self::default(); }
    fn transition_release(&mut self) -> Vec<InputAction> {
        let Some(profile) = self.profile.as_ref() else { self.down.clear(); return vec![] };
        let releases: Vec<String> = profile.actions.iter()
            .filter(|a| a.phases & 2 != 0 && self.down.contains(&a.id))
            .map(|a| a.id.clone()).collect();
        self.down.clear();
        releases.into_iter().map(|id| self.out(id, false)).collect()
    }
    fn out(&mut self, id: String, pressed: bool) -> InputAction {
        self.sequence = self.sequence.saturating_add(1);
        InputAction {id, pressed, sequence:self.sequence}
    }
    pub fn apply_state(&mut self, wire: &str) -> Result<Vec<InputAction>, &'static str> {
        let (revision, state) = parse_state(wire)?;
        if self.state_revision.is_some_and(|r| revision < r) { return Ok(vec![]); }
        let releases = if self.state != state { self.transition_release() } else { vec![] };
        self.state = state;
        self.state_revision = Some(revision);
        self.ready = true;
        Ok(releases)
    }
    pub fn apply_profile(&mut self, wire: &str) -> Result<Vec<InputAction>, &'static str> {
        let next = parse_profile(wire)?;
        if self.profile.as_ref().is_some_and(|p| next.revision < p.revision) {
            return Ok(vec![]);
        }
        let releases = self.transition_release();
        self.profile = Some(next);
        Ok(releases)
    }
    pub fn handle_key(&mut self, key: &str, pressed: bool, playing: bool) -> Vec<InputAction> {
        if !playing { return self.transition_release(); }
        let Some(profile) = self.profile.as_ref() else {return vec![]};
        if !self.ready || self.state != profile.state {return vec![];}
        let delta:Vec<(String, u8)> = profile.actions.iter()
            .filter(|a| a.binding.matches(key) && self.down.contains(&a.id) != pressed)
            .map(|a| (a.id.clone(), a.phases)).collect();
        let mut result=Vec::new();
        for (id, phases) in delta {
            if pressed { self.down.insert(id.clone()); } else { self.down.remove(&id); }
            if (pressed && phases & 1 != 0) || (!pressed && phases & 2 != 0) {
                result.push(self.out(id,pressed));
            }
        }
        result
    }
    pub fn release_all(&mut self) -> Vec<InputAction> { self.transition_release() }
    pub fn heartbeat(&mut self, playing: bool) -> Vec<InputAction> {
        if !playing { return self.transition_release(); }
        self.tick = self.tick.wrapping_add(1);
        if self.tick % 5 != 0 || !self.ready {return vec![];}
        let Some(profile) = self.profile.as_ref() else {return vec![]};
        if self.state != profile.state {return vec![];}
        // Match the Fabric client's bounded tractor heartbeat; don't replay
        // general actions (shooting, reloading, interacting).
        let ids:Vec<String> = profile.actions.iter()
            .filter(|a| a.phases & 1 != 0 && self.down.contains(&a.id))
            .filter(|a| a.id.starts_with("tractor_prototipo:conduccion."))
            .filter(|a| matches!(a.id.rsplit('.').next(),Some("forward" | "reverse" | "left" | "right" | "brake")))
            .map(|a| a.id.clone()).collect();
        ids.into_iter().map(|id| self.out(id,true)).collect()
    }
    pub fn profile_id(&self) -> Option<&str> { self.profile.as_ref().map(|p| p.id.as_str()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> &'static str {"8|3|virgoland:drive|DRIVING|Q29udHJvbGVz|2|tractor_prototipo:conduccion.forward,QXZhbnphcg,k:87,3;virgoland:horn,Qm9jaW5h,m:1,1"}
    #[test]
    fn parses_bridge_profile_and_rejects_replays_and_bad_wire() {
        let p=parse_profile(profile()).unwrap();
        assert_eq!(p.actions.len(),2);
        assert!(p.actions[0].binding.matches("key.keyboard.w"));
        assert!(p.actions[1].binding.matches("key.mouse.right"));
        for wire in ["8|1|virgoland:p|DRIVING|SGl|33|", "8|1|virgoland:p|BAD|SGl|0|",
            "8|1|virgoland:p|DRIVING|SGl|1|oops,SGk,k:1234567,1",
            "8|1|virgoland:p|DRIVING|SGl|1|x,SGk,k:65,7"] {
            assert!(parse_profile(wire).is_err());
        }
        assert_eq!(parse_view("8|3.25").unwrap(),3.25);
        for wire in ["8|0","8|NaN","8|11","9|2"] {assert!(parse_view(wire).is_err());}
    }
    #[test]
    fn press_release_focus_loss_and_tracked_sequence() {
        let mut s=ControlsSession::default();
        s.apply_state("8|1|DRIVING").unwrap();
        s.apply_profile(profile()).unwrap();
        assert_eq!(s.handle_key("key.keyboard.w",true,true),
            [InputAction{id:"tractor_prototipo:conduccion.forward".into(),pressed:true,sequence:1}]);
        assert!(s.handle_key("key.keyboard.w",true,true).is_empty());
        for _ in 0..4 {assert!(s.heartbeat(true).is_empty());}
        assert_eq!(s.heartbeat(true)[0].sequence,2);
        let releases=s.release_all();
        assert_eq!(releases[0].sequence,3);
        assert!(!releases[0].pressed);
        assert!(s.handle_key("key.keyboard.w",false,true).is_empty());
        s.handle_key("key.keyboard.w",true,true);
        let dropped=s.apply_state("8|2|FOOTBALL").unwrap();
        assert_eq!(dropped.len(),1);
        assert!(!dropped[0].pressed);
    }
}
