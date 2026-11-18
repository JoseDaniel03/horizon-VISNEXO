//! Server-authoritative VSClient control profiles for Horizon/MCRust.
//! Implements the Java Bridge protocol 8 state/profile/input contract.
//! Pure Rust, no Fabric/GLFW, so protocol tests run without the game.
use std::collections::{HashMap, HashSet};

pub const CHANNEL_STATE: &str = "virgoland:state";
pub const CHANNEL_PROFILE: &str = "virgoland:profile";
pub const CHANNEL_INPUT: &str = "virgoland:input";
pub const CHANNEL_VIEW: &str = "virgoland:view";
const MAX_PROFILE_BYTES: usize = 32_768;
const MAX_ACTIONS: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binding { Key(i32), Mouse(u8), None }
impl Binding {
    pub fn parse(wire: &str) -> Result<Self, &'static str> {
        let (device, code) = wire.split_once(':').ok_or("Missing binding separator")?;
        let code = code.parse::<i32>().map_err(|_| "Invalid binding code")?;
        match (device, code) {
            ("k", 0..=512) => Ok(Self::Key(code)),
            ("m", 0..=2) => Ok(Self::Mouse(code as u8)),
            ("n", -1) => Ok(Self::None),
            _ => Err("Unsupported control binding"),
        }
    }
    pub fn to_wire(&self) -> String {
        match self { Self::Key(k) => format!("k:{k}"), Self::Mouse(m) => format!("m:{m}"), Self::None => "n:-1".into() }
    }
    /// The legacy Bridge key codes correspond to GLFW integer codes; Horizon
    /// exposes neutral Minecraft key names, so convert only at this boundary.
    pub fn from_key(key: &str) -> Option<Self> {
        let mouse = match key {
            "key.mouse.left" | "key.mouse.0" | "key.mouse.1" => Some(0),
            "key.mouse.right" | "key.mouse.2" => Some(1),
            "key.mouse.middle" | "key.mouse.3" => Some(2),
            _ => None,
        };
        if let Some(m) = mouse { return Some(Self::Mouse(m)); }
        let k=key.strip_prefix("key.keyboard.")?;
        if k.len()==1 {
            let b=k.as_bytes()[0];
            if b.is_ascii_lowercase() {return Some(Self::Key((b-32) as i32));}
            if b.is_ascii_digit() {return Some(Self::Key(b as i32));}
        }
        if let Some(i)=k.strip_prefix('f').and_then(|x|x.parse::<i32>().ok()) {
            if (1..=12).contains(&i) {return Some(Self::Key(289+i));}
        }
        let n=match k {
            "space"=>32, "escape"=>256, "enter" | "return"=>257, "tab"=>258,
            "backspace"=>259, "insert"=>260, "delete"=>261, "right"=>262, "left"=>263,
            "down"=>264, "up"=>265, "left.shift"=>340, "left.control"=>341,
            "left.alt"=>342, "right.shift"=>344, "right.control"=>345, "right.alt"=>346,
            _=>return None,
        };
        Some(Self::Key(n))
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
    pub display_name: String,
}
#[derive(Clone, Debug)]
pub struct Profile {
    pub revision: u64,
    pub id: String,
    pub state: String,
    pub display_name: String,
    pub actions: Vec<Action>,
}
/// Bounded base64url decoder for the Bridge's untrusted display labels.
fn decode_label(raw: &str) -> Result<String, &'static str> {
    if raw.len() > 256 || raw.bytes().any(|b| !b.is_ascii_alphanumeric() && !matches!(b,b'-'|b'_'|b'=')) {
        return Err("Invalid encoded label");
    }
    let mut out=Vec::new();
    let mut bits=0_u32;
    let mut count=0_u8;
    let mut padding=false;
    for c in raw.bytes() {
        if c==b'=' {padding=true;continue;}
        if padding {return Err("Invalid label padding");}
        let v=match c {b'A'..=b'Z'=>c-b'A',b'a'..=b'z'=>c-b'a'+26,
            b'0'..=b'9'=>c-b'0'+52,b'-'=>62,b'_'=>63,_=>return Err("Invalid encoded label")};
        bits=(bits<<6)|u32::from(v);
        count+=6;
        if count>=8 {count-=8;out.push((bits>>count) as u8);bits&=(1_u32<<count)-1;}
    }
    if count==6 || (count>0 && bits!=0) {return Err("Bad label bit padding");}
    let text=String::from_utf8(out).map_err(|_|"Invalid label UTF-8")?;
    if text.chars().any(|c|c.is_control()) || text.chars().count()>100 {return Err("Unsafe label");}
    Ok(text)
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
        return Ok(Profile { revision, id: "-".into(), state: "OVERWORLD".into(), display_name: "Sin perfil".into(), actions: vec![] });
    }
    if p.len() != 7 || !valid_id(p[2]) || !valid_state(p[3]) {
        return Err("Bad profile identity");
    }
    // The name fields are URL-safe base64 from the Bridge. We do not display
    // them in this headless version; still reject malformed/unbounded wire.
    let valid_b64 = |s: &str| s.len() <= 256
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'=');
    let display_name=decode_label(p[4])?;
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
        actions.push(Action {id: col[0].into(), binding: Binding::parse(col[2])?, phases, display_name: decode_label(col[1])?});
    }
    Ok(Profile {revision, id:p[2].into(), state:p[3].into(), display_name, actions})
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputAction { pub id: String, pub pressed: bool, pub sequence: u64 }
#[derive(Default)]
pub struct ControlsSession {
    state: String,
    state_revision: Option<u64>,
    profile: Option<Profile>,
    overrides: HashMap<(String,String),Binding>,
    ui_page: usize,
    capture: Option<String>,
    down: HashSet<String>,
    physical_down: HashSet<String>,
    touch_held: Option<String>,
    virtual_enabled: bool,
    virtual_page: usize,
    sequence: u64,
    tick: u64,
    ready: bool,
}
impl ControlsSession {
    pub fn reset(&mut self) { self.state.clear(); self.state_revision=None; self.profile=None; self.down.clear(); self.physical_down.clear(); self.touch_held=None; self.virtual_page=0; self.sequence=0; self.tick=0; self.ready=false; self.ui_page=0; self.capture=None; }
    fn transition_release(&mut self) -> Vec<InputAction> {
        let Some(profile) = self.profile.as_ref() else { self.down.clear();self.physical_down.clear();self.touch_held=None; return vec![] };
        let releases: Vec<String> = profile.actions.iter()
            .filter(|a| a.phases & 2 != 0 && self.down.contains(&a.id))
            .map(|a| a.id.clone()).collect();
        self.down.clear();
        self.physical_down.clear();
        self.touch_held=None;
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
        self.ui_page=0;
        self.virtual_page=0;
        self.capture=None;
        Ok(releases)
    }
    pub fn handle_key(&mut self, key: &str, pressed: bool, playing: bool) -> Vec<InputAction> {
        if !playing { return self.transition_release(); }
        let Some(profile) = self.profile.as_ref() else {return vec![]};
        if !self.ready || self.state != profile.state {return vec![];}
        let delta:Vec<(String, u8)> = profile.actions.iter()
            .filter(|a| self.effective_binding(&profile.id,a).matches(key) && self.physical_down.contains(&a.id) != pressed)
            .map(|a| (a.id.clone(), a.phases)).collect();
        let mut result=Vec::new();
        for (id, phases) in delta {
            let was_down=self.down.contains(&id);
            if pressed {self.physical_down.insert(id.clone());}
            else {self.physical_down.remove(&id);}
            let now_down=self.physical_down.contains(&id)
                || self.touch_held.as_deref()==Some(id.as_str());
            if was_down==now_down {continue;}
            if now_down {self.down.insert(id.clone());}
            else {self.down.remove(&id);}
            if (now_down && phases & 1 != 0) || (!now_down && phases & 2 != 0) {
                result.push(self.out(id,now_down));
            }
        }
        result
    }
    /// Software buttons share semantic actions and the server's sequence with
    /// physical keys. An action stays held until both sources are released.
    pub fn touch_press(&mut self,id:&str)->Vec<InputAction>{
        if self.touch_held.as_deref()==Some(id){return vec![];}
        let mut out=self.touch_release();
        let Some(p)=self.profile.as_ref() else{return out;};
        if !self.ready || self.state!=p.state || !self.virtual_enabled {return out;}
        let Some(phases)=p.actions.iter().find(|a|a.id==id).map(|a|a.phases) else{return out;};
        let was_down=self.down.contains(id);
        self.touch_held=Some(id.to_owned());
        self.down.insert(id.to_owned());
        if !was_down && phases & 1!=0 {out.push(self.out(id.to_owned(),true));}
        out
    }
    pub fn touch_release(&mut self)->Vec<InputAction>{
        let Some(id)=self.touch_held.take() else{return vec![];};
        if self.physical_down.contains(&id) {return vec![];}
        if !self.down.remove(&id) {return vec![];}
        let phases=self.profile.as_ref().and_then(|p|p.actions.iter()
            .find(|a|a.id==id).map(|a|a.phases)).unwrap_or(0);
        if phases & 2 !=0 {vec![self.out(id,false)]} else {vec![]}
    }
    pub fn touch_active(&self)->bool{self.touch_held.is_some()}
    pub fn virtual_enabled(&self)->bool{self.virtual_enabled}
    pub fn set_virtual_enabled(&mut self,enabled:bool)->Vec<InputAction>{
        let releases=if !enabled{self.touch_release()}else{vec![]};
        self.virtual_enabled=enabled;
        releases
    }
    pub fn virtual_page(&self)->usize{self.virtual_page}
    pub fn touch_page_step(&mut self,total:usize,page_size:usize){
        let last=total.saturating_sub(1)/page_size.max(1);
        self.virtual_page=if self.virtual_page>=last{0}else{self.virtual_page+1};
    }
    pub fn virtual_actions(&self)->Vec<(String,String,bool)>{
        if !self.virtual_enabled || !self.ready{return vec![];}
        let Some(p)=self.profile.as_ref() else{return vec![];};
        if self.state!=p.state{return vec![];}
        p.actions.iter().map(|a|{
            (a.id.clone(),a.display_name.clone(),self.down.contains(&a.id))
        }).collect()
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
    /// Only an active server profile can claim physical mouse/movement inputs.
    pub fn claimed_mouse(&self, button:i32)->bool {
        let Some(p)=self.profile.as_ref() else {return false};
        self.ready && self.state==p.state && p.actions.iter()
            .any(|a|matches!(self.effective_binding(&p.id,a),Binding::Mouse(n) if i32::from(n)==button))
    }
    /// Don't let Vanilla movement leak through a server-owned vehicle binding.
    /// Keyboard and gamepad movement still remain user controlled elsewhere.
    pub fn claims_move(&self, keys:&[i32])->bool {
        let Some(p)=self.profile.as_ref() else {return false};
        self.ready && self.state==p.state && p.actions.iter()
            .any(|a|matches!(self.effective_binding(&p.id,a),Binding::Key(k) if keys.contains(&k)))
    }
    pub fn effective_binding(&self, profile_id: &str, action: &Action) -> Binding {
        self.overrides.get(&(profile_id.to_owned(),action.id.clone()))
            .cloned().unwrap_or_else(||action.binding.clone())
    }
    pub fn current_profile(&self) -> Option<&Profile> {self.profile.as_ref()}
    pub fn ui_page(&self) -> usize {self.ui_page}
    pub fn capture_id(&self) -> Option<&str> {self.capture.as_deref()}
    pub fn page_step(&mut self, amount: i32, per_page: usize) {
        let count=self.profile.as_ref().map_or(0,|p|p.actions.len());
        let last=count.saturating_sub(1)/per_page.max(1);
        if amount<0 {self.ui_page=self.ui_page.saturating_sub(1);}
        else {self.ui_page=(self.ui_page+1).min(last);}
    }
    pub fn begin_capture(&mut self, id: &str) -> bool {
        let Some(p)=self.profile.as_ref() else{return false};
        if !p.actions.iter().any(|a|a.id==id){return false;}
        self.capture=Some(id.into());true
    }
    pub fn cancel_capture(&mut self){self.capture=None;}
    pub fn apply_capture(&mut self, key: &str) -> Result<bool,&'static str> {
        let Some(id)=self.capture.take() else{return Ok(false)};
        if key=="key.keyboard.escape" {return Ok(false);}
        let binding=if matches!(key, "key.keyboard.delete"|"key.keyboard.backspace") {
            Binding::None
        } else {Binding::from_key(key).ok_or("Unsupported key binding")?};
        let Some(p)=self.profile.as_ref() else {return Ok(false)};
        let profile_id=p.id.clone();
        if !p.actions.iter().any(|a|a.id==id) {return Ok(false);}
        self.down.clear();
        // Eliminate duplicate assignments within this profile: the edited
        // action takes precedence, leaving the previous binding unbound.
        if binding!=Binding::None {
            let conflicts:Vec<String>=p.actions.iter()
                .filter(|a|a.id!=id && self.effective_binding(&profile_id,a)==binding)
                .map(|a|a.id.clone()).collect();
            for other in conflicts {self.overrides.insert((profile_id.clone(),other),Binding::None);}
        }
        self.overrides.insert((profile_id,id),binding);
        Ok(true)
    }
    pub fn clear_profile_overrides(&mut self){
        if let Some(p)=self.profile.as_ref(){self.overrides.retain(|(id,_),_|id!=&p.id);}
        self.down.clear();self.capture=None;
    }
    pub fn bindings_wire(&self) -> String {
        let mut rows:Vec<String>=self.overrides.iter().map(|((p,a),b)|format!("{p}|{a}|{}",b.to_wire())).collect();
        rows.sort();
        rows.join("\n")
    }
    pub fn load_bindings(&mut self,wire:&str)->Result<(),&'static str>{
        if wire.len()>32768{return Err("Settings too large");}
        let mut next=HashMap::new();
        for row in wire.lines(){
            if row.is_empty(){continue;}
            let cols:Vec<&str>=row.split('|').collect();
            if cols.len()!=3 || !valid_id(cols[0]) || !valid_id(cols[1]) || next.len()>=256{
                return Err("Invalid saved settings");
            }
            next.insert((cols[0].into(),cols[1].into()),Binding::parse(cols[2])?);
        }
        self.overrides=next;self.down.clear();Ok(())
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
    fn binds_persist_per_profile_and_reject_unsupported_inputs() {
        let mut s=ControlsSession::default();
        s.apply_state("8|1|DRIVING").unwrap();
        s.apply_profile(profile()).unwrap();
        assert!(s.begin_capture("tractor_prototipo:conduccion.forward"));
        assert!(s.apply_capture("key.keyboard.t").unwrap());
        assert_eq!(s.handle_key("key.keyboard.w",true,true).len(),0);
        assert_eq!(s.handle_key("key.keyboard.t",true,true).len(),1);
        let saved=s.bindings_wire();
        let mut new=ControlsSession::default();
        new.load_bindings(&saved).unwrap();
        new.apply_state("8|1|DRIVING").unwrap();
        new.apply_profile(profile()).unwrap();
        assert_eq!(new.handle_key("key.keyboard.t",true,true).len(),1);
        assert!(new.load_bindings("not-valid").is_err());
        assert_eq!(Binding::from_key("key.keyboard.f12"),Some(Binding::Key(301)));
        assert_eq!(Binding::from_key("key.keyboard.delete"),Some(Binding::Key(261)));
        assert!(new.begin_capture("tractor_prototipo:conduccion.forward"));
        assert!(new.apply_capture("key.keyboard.delete").unwrap());
        assert_eq!(new.handle_key("key.keyboard.t",true,true).len(),0);
        new.clear_profile_overrides();
        assert_eq!(new.handle_key("key.keyboard.w",true,true).len(),1);
    }
    #[test]
    fn preserves_bindings_across_session_resets_and_rejects_corrupt_labels(){
        let mut s=ControlsSession::default();
        s.load_bindings("virgoland:drive|virgoland:horn|k:70").unwrap();
        s.reset();
        assert!(s.bindings_wire().contains("virgoland:horn|k:70"));
        assert!(parse_profile("8|2|virgoland:drive|DRIVING|$|0|").is_err());
        assert_eq!(decode_label("Q2FtacOzbg").unwrap(),"Camión");
    }
    #[test]
    fn virtual_touch_and_keyboard_share_action_until_both_release(){
        let mut s=ControlsSession::default();
        s.apply_state("8|1|DRIVING").unwrap();
        s.apply_profile(profile()).unwrap();
        s.set_virtual_enabled(true);
        let id="tractor_prototipo:conduccion.forward";
        assert_eq!(s.touch_press(id).len(),1);
        assert!(s.handle_key("key.keyboard.w",true,true).is_empty());
        assert!(s.touch_release().is_empty());
        let release=s.handle_key("key.keyboard.w",false,true);
        assert_eq!(release.len(),1);
        assert!(!release[0].pressed);
        assert_eq!(s.touch_press(id).len(),1);
        let closed=s.set_virtual_enabled(false);
        assert_eq!(closed.len(),1);
        assert!(!closed[0].pressed);
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
