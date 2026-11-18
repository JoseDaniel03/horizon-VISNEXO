//! Platform-neutral VSClient protocol 8 and camera cue timeline for MCRust.
//! Keep this independent of Fabric, GLFW, Java and Horizon's rendering internals.
use std::time::Instant;

pub const PROTOCOL: u32 = 8;
pub const CHANNEL_HELLO: &str = "virgoland:hello";
pub const CHANNEL_AUTH: &str = "virgoland:auth";
pub const CHANNEL_CAMERA: &str = "virgoland:camera_v1";
pub const CHANNEL_CINEMATIC: &str = "virgoland:cinematic_v3";
pub const CHANNEL_INPUT: &str = "virgoland:input";
pub const MAX_WIRE_BYTES: usize = 32_768;
pub const MAX_CUE_BYTES: usize = 8_192;
pub const MAX_KEYS: usize = 64;
pub const MAX_EFFECTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub pos: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub fov: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub seconds: f64,
    pub pose: Pose,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub id: String,
    pub begin: f64,
    pub duration: f64,
    pub fade_in: f64,
    pub fade_out: f64,
    pub strength: f64,
}
/// Supported software overlays on the final game GUI. Unknown server effect
/// identifiers are intentionally ignored until their native renderer exists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Overlay {pub rgb: u32, pub alpha: f32, pub letterbox: bool}

#[derive(Clone, Debug)]
pub struct CameraCue {
    pub revision: u64,
    pub keys: Vec<Key>,
    pub effects: Vec<Effect>,
}
impl CameraCue {
    pub fn duration(&self) -> f64 {
        self.keys.last().map_or(0.0, |k| k.seconds)
    }
    pub fn sample(&self, seconds: f64) -> Pose {
        let first = &self.keys[0];
        if !seconds.is_finite() || seconds <= first.seconds { return first.pose; }
        let last = &self.keys[self.keys.len()-1];
        if seconds >= last.seconds { return last.pose; }
        let high = self.keys.partition_point(|k| k.seconds <= seconds);
        let before = &self.keys[high - 1];
        let after = &self.keys[high];
        let fraction = ((seconds - before.seconds) / (after.seconds - before.seconds)).clamp(0.0,1.0);
        let t = fraction * fraction * (3.0 - 2.0 * fraction);
        let lerp = |a:f64,b:f64| a + (b-a)*t;
        let angle = |a:f32,b:f32| {
            let delta = ((b as f64 - a as f64 + 180.0).rem_euclid(360.0)) - 180.0;
            (a as f64 + delta*t) as f32
        };
        Pose {
            pos: std::array::from_fn(|i| lerp(before.pose.pos[i],after.pose.pos[i])),
            yaw:angle(before.pose.yaw,after.pose.yaw),
            pitch:angle(before.pose.pitch,after.pose.pitch),
            roll:angle(before.pose.roll,after.pose.roll),
            fov:lerp(before.pose.fov as f64,after.pose.fov as f64) as f32,
        }
    }
}
fn number(s: &str) -> Result<f64, &'static str> {
    let value = s.parse::<f64>().map_err(|_|"Invalid numeric value")?;
    if !value.is_finite() {return Err("Non-finite numeric value");}
    Ok(value)
}
pub fn parse_camera_cue(payload: &str) -> Result<CameraCue,&'static str> {
    if payload.is_empty() || payload.len() > MAX_CUE_BYTES {return Err("Camera cue too large");}
    let parts:Vec<&str> = payload.split('|').collect();
    if parts.len()!=4 || parts[0]!="v1" {return Err("Unknown VSClient camera cue version");}
    let revision = parts[1].parse::<u64>().map_err(|_|"Invalid revision")?;
    let frames:Vec<&str> = parts[2].split(';').collect();
    if frames.is_empty() || frames.len()>MAX_KEYS {return Err("Invalid keyframe count");}
    let mut keys=Vec::with_capacity(frames.len());
    let mut last_time=-1.0;
    for frame in frames {
        let cols:Vec<&str> = frame.split(',').collect();
        if cols.len()!=8 {return Err("Camera keyframe must contain 8 values");}
        let vals:Vec<f64> = cols.iter().map(|v|number(v)).collect::<Result<_,_>>()?;
        if vals[0]<0.0 || vals[0]<=last_time {return Err("Keyframes are not strictly increasing");}
        if vals[1..=3].iter().any(|v|v.abs()>30_000_000.0) {
            return Err("Camera outside world coordinate bounds");
        }
        if !(1.0..179.0).contains(&vals[7]) {return Err("Invalid camera FOV");}
        // Match Java CameraPose's finite angles (not forced to vanilla limits).
        if vals[4..=6].iter().any(|v|v.abs()>1.0e6) {return Err("Camera rotation exceeds safe range");}
        keys.push(Key { seconds:vals[0], pose:Pose {
            pos:[vals[1],vals[2],vals[3]],yaw:vals[4] as f32,
            pitch:vals[5] as f32, roll:vals[6] as f32, fov:vals[7] as f32
        }});
        last_time=vals[0];
    }
    let mut effects=Vec::new();
    if !parts[3].is_empty() {
        let columns:Vec<&str>=parts[3].split(';').collect();
        if columns.len()>MAX_EFFECTS{return Err("Too many render effects");}
        for entry in columns {
            let cols:Vec<&str>=entry.split(',').collect();
            if cols.len()!=6 || cols[0].len()>80 || cols[0].is_empty() {
                return Err("Invalid effect description");
            }
            let n:Vec<f64>=cols[1..].iter().map(|v|number(v)).collect::<Result<_,_>>()?;
            if n[0]<0.0 || n[1]<0.0 || n[2]<0.0 || n[3]<0.0 || n[4]<0.0 {
                return Err("Negative effect envelope");
            }
            effects.push(Effect {id:cols[0].into(),begin:n[0],duration:n[1],
                fade_in:n[2],fade_out:n[3],strength:n[4]});
        }
    }
    Ok(CameraCue{revision,keys,effects})
}

pub fn decode_string(data: &[u8], max:usize)->Result<&str,&'static str> {
    let mut result=0u32;
    let mut offset:usize=0;
    for i in 0..5 {
        let b=*data.get(i).ok_or("Incomplete Minecraft VarInt")?;
        if i==4 && b>0x0f {return Err("VarInt overflow");}
        result|=u32::from(b&0x7f)<<(7*i);
        offset+=1;
        if b&0x80==0 {
            let len=usize::try_from(result).map_err(|_|"Length overflow")?;
            if len>max || len>MAX_WIRE_BYTES || offset.checked_add(len)!=Some(data.len()) {
                return Err("Invalid UTF-8 string length");
            }
            return std::str::from_utf8(&data[offset..]).map_err(|_|"Invalid UTF-8");
        }
    }
    Err("Minecraft VarInt too long")
}
pub fn encode_string(s:&str)->Result<Vec<u8>,&'static str> {
    let src=s.as_bytes();
    if src.len()>MAX_WIRE_BYTES {return Err("String is too large");}
    let mut n=src.len() as u32;
    let mut out=Vec::with_capacity(src.len()+5);
    loop {
        let mut b=(n&0x7f) as u8;
        n >>= 7;
        if n!=0 {b|=0x80;}
        out.push(b);
        if n==0 {break;}
    }
    out.extend_from_slice(src);
    Ok(out)
}
/// Only advertise capabilities already backed by the Horizon native adapter.
/// Send a short-lived one-use bridge ticket supplied by VIS Nexo. Never log,
/// persist or expose it to camera packets. Restrict to a safe ASCII alphabet:
/// the existing Java Bridge serializes the ticket into a JSON string.
pub fn auth_ticket(ticket:&str) -> Result<Vec<u8>,&'static str> {
    if ticket.is_empty() || ticket.len()>2048
        || !ticket.bytes().all(|b|b.is_ascii_alphanumeric() || matches!(b,b'-'|b'_'|b'.')) {
        return Err("Invalid launcher bridge ticket");
    }
    encode_string(&format!("8|{ticket}"))
}
pub fn hello() -> Vec<u8> {
    encode_string("8|0.3.0-mcrust|CAMERA_V1,CONTROLS_V1,CONTROLS_UI_V1,VIEW_V1").expect("constant is safe")
}
#[derive(Default)]
pub struct CameraSession {
    current: Option<(Instant,CameraCue)>,
    last_revision: Option<u64>,
}
impl CameraSession {
    pub fn accept(&mut self,text:&str,now:Instant)->Result<(),&'static str> {
        let cue=parse_camera_cue(text)?;
        if self.last_revision.is_some_and(|r|cue.revision<=r){return Err("Stale camera revision");}
        self.last_revision=Some(cue.revision);
        self.current=Some((now,cue));
        Ok(())
    }
    pub fn sample(&mut self, now:Instant)->Option<Pose> {
        let (start,cue)=self.current.as_ref()?;
        let elapsed=now.saturating_duration_since(*start).as_secs_f64();
        if elapsed>cue.duration() {
            self.current=None;
            return None;
        }
        Some(cue.sample(elapsed))
    }
    pub fn overlays(&mut self, now: Instant) -> Vec<Overlay> {
        let Some((started,cue)) = self.current.as_ref() else {return vec![]};
        let elapsed=now.saturating_duration_since(*started).as_secs_f64();
        if elapsed>cue.duration(){return vec![];}
        let mut overlays=Vec::new();
        for e in &cue.effects {
            if elapsed<e.begin || elapsed>e.begin+e.duration {continue;}
            let (rgb,letterbox)=match e.id.as_str() {
                "cinema:fade_black" | "cinema:black" => (0x000000,false),
                "cinema:fade_white" | "cinema:flash_white" => (0xffffff,false),
                "cinema:letterbox" => (0x000000,true),
                _ => continue,
            };
            let local=elapsed-e.begin;
            let enter=if e.fade_in>0.0 {(local/e.fade_in).clamp(0.0,1.0)} else {1.0};
            let exit=if e.fade_out>0.0 {
                ((e.duration-local)/e.fade_out).clamp(0.0,1.0)
            } else {1.0};
            let alpha=(e.strength*enter.min(exit)).clamp(0.0,1.0) as f32;
            if alpha>0.0 {overlays.push(Overlay{rgb,alpha,letterbox});}
        }
        overlays
    }
    pub fn reset(&mut self){self.current=None;self.last_revision=None;}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn java_protocol_8_string_codec_matches_minecraft_varint() {
        let data=encode_string("8|MCRust|CAMERA_V1").unwrap();
        assert_eq!(data[0],18);
        assert_eq!(decode_string(&data,8192).unwrap(),"8|MCRust|CAMERA_V1");
        let big="á".repeat(100);
        let encoded=encode_string(&big).unwrap();
        assert_eq!(decode_string(&encoded,8192).unwrap(),big);
        for value in [vec![],vec![0x81],vec![0x80,0x80,0x80,0x80,0x80],vec![4,65,66],
            vec![0xff,0xff,0xff,0xff,0xff,0],vec![1,0xff]] {
            assert!(decode_string(&value,8192).is_err(),"{value:?}");
        }
    }
    #[test]
    fn launcher_ticket_is_validated_and_sent_in_legacy_auth_wire_format() {
        assert_eq!(decode_string(&auth_ticket("short_LAUNCHER-ticket.1").unwrap(),MAX_WIRE_BYTES)
            .unwrap(),"8|short_LAUNCHER-ticket.1");
        for invalid in ["","token with spaces","\"injection", "a/b", "line\nbreak"] {
            assert!(auth_ticket(invalid).is_err(),"{invalid:?}");
        }
        assert!(auth_ticket(&"x".repeat(2049)).is_err());
    }
    #[test]
    fn camera_cue_interpolates_and_wraps_yaw() {
        let cue=parse_camera_cue("v1|5|0,0,64,0,350,0,0,70;2,10,64,20,10,30,0,90|").unwrap();
        let half=cue.sample(1.0);
        assert_eq!(half.pos,[5.0,64.0,10.0]);
        assert!((half.yaw-360.0).abs()<0.01);
        assert!((half.fov-80.0).abs()<0.01);
        assert_eq!(cue.duration(),2.0);
    }
    #[test]
    fn sends_only_advertised_capabilities_and_bounds_visual_fades() {
        assert_eq!(decode_string(&hello(),MAX_WIRE_BYTES).unwrap(),
            "8|0.2.0-mcrust|CAMERA_V1,CONTROLS_V1,VIEW_V1");
        let now=Instant::now();
        let mut c=CameraSession::default();
        c.accept("v1|1|0,0,64,0,0,0,0,70;4,0,64,0,0,0,0,70|cinema:fade_black,0,4,1,1,0.75;cinema:unknown,0,4,0,0,1",now).unwrap();
        assert!(c.overlays(now).is_empty());
        let mid=c.overlays(now+std::time::Duration::from_secs(2));
        assert_eq!(mid.len(),1);
        assert_eq!(mid[0],Overlay{rgb:0,alpha:0.75,letterbox:false});
        assert!(c.overlays(now+std::time::Duration::from_secs(5)).is_empty());
    }
    #[test]
    fn rejects_malformed_oversized_nonfinite_and_replayed_cues() {
        for text in ["", "v2|1|0,0,0,0,0,0,0,70|", "v1|1|0,0,0,0,0,0,0,180|",
            "v1|1|0,NaN,0,0,0,0,0,70|","v1|1|0,0,0,0,0,0,0,70;0,1,0,0,0,0,0,70|",
            "v1|1|-1,0,0,0,0,0,0,70|","v1|1|0,30000001,0,0,0,0,0,70|"] {
            assert!(parse_camera_cue(text).is_err(),"{text}");
        }
        let now=Instant::now();
        let mut session=CameraSession::default();
        let cue="v1|10|0,0,64,0,30,0,0,70;1,2,64,2,40,0,0,75|";
        assert!(session.accept(cue,now).is_ok());
        assert!(session.accept(cue,now).is_err());
        assert!(session.sample(now+std::time::Duration::from_millis(500)).is_some());
        assert!(session.sample(now+std::time::Duration::from_secs(2)).is_none());
        session.reset();
        assert!(session.accept(cue,now).is_ok());
    }
}
