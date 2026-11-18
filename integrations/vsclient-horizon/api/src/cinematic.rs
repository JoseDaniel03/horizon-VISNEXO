//! Strict, bounded decoder for VSClient Fabric's CINEMATIC_V3 scene windows.
//! The native renderer currently exposes the primary camera and software GUI
//! effects; additional GPU views and entity avatars still require a renderer.
use std::collections::HashSet;
use std::time::Instant;
use horizon_client::mod_api::serde_json::{self,Value};
use super::core::{CameraCue,Key,Pose};

pub const CHANNEL: &str="virgoland:cinematic_v3";
const MAX_BYTES: usize=30_000;

#[derive(Clone,Copy,Debug,PartialEq)]
pub struct Rect {pub x:f32,pub y:f32,pub w:f32,pub h:f32}
impl Rect {
    fn full()->Self{Self{x:0.0,y:0.0,w:1.0,h:1.0}}
}
#[derive(Clone,Debug)]
pub struct Fade {pub color:u32,pub amount:f32}
#[derive(Clone,Debug)]
pub struct View {pub id:String,pub rect:Rect,pub cue:CameraCue,pub opacity:f32,pub fade:Option<Fade>}
#[derive(Clone,Debug)]
pub struct Caption {
    pub id:String,pub text:String,pub font:String,pub x:f32,pub y:f32,
    pub size:f32,pub color:u32,pub opacity:f32,pub rotation:f32,pub align:String
}
#[derive(Clone,Debug)]
pub struct Effect {pub kind:String,pub amount:f32,pub color:u32,pub rect:Rect}
#[derive(Clone,Debug)]
pub struct Actor {pub id:String,pub profile:String,pub entity_id:Option<String>,
    pub entity_type:String,pub cue:CameraCue,pub scales:Vec<f32>}
impl Actor {
    /// Sample the same linear scale between ordered keys as Java.
    /// Pose sampling uses the camera's shortest-angle interpolation.
    pub fn sample(&self,seconds:f64)->(Pose,f32) {
        let pose=self.cue.sample(seconds);
        let hi=self.cue.keys.partition_point(|k|k.seconds<=seconds);
        let before=hi.saturating_sub(1);
        let Some(next)=self.cue.keys.get(hi) else {return (pose,self.scales[before]);};
        let current=&self.cue.keys[before];
        let factor=((seconds-current.seconds)/(next.seconds-current.seconds)).clamp(0.0,1.0) as f32;
        (pose,self.scales[before]+(self.scales[hi]-self.scales[before])*factor)
    }
}
#[derive(Clone,Debug)]
pub struct Scene {pub revision:u64,pub id:String,pub stop:bool,pub views:Vec<View>,
    pub actors:Vec<Actor>,pub captions:Vec<Caption>,pub effects:Vec<Effect>}
fn field<'a>(v:&'a Value,key:&str)->Result<&'a Value,&'static str>{
    v.get(key).ok_or("Missing cinematic field")
}
fn string<'a>(v:&'a Value)->Result<&'a str,&'static str>{
    v.as_str().ok_or("Expected cinematic string")
}
fn number(v:&Value)->Result<f64,&'static str>{
    let x=v.as_f64().ok_or("Expected cinematic number")?;
    if !x.is_finite(){return Err("Nonfinite cinematic number");}
    Ok(x)
}
fn finite32(v:&Value,min:f64,max:f64)->Result<f32,&'static str>{
    let x=number(v)?;
    if !(min..=max).contains(&x){return Err("Cinematic number outside limits");}
    Ok(x as f32)
}
fn color(v:&Value)->Result<u32,&'static str>{
    let n=v.as_i64().ok_or("Bad scene color")?;
    if !(-2_147_483_648..=4_294_967_295).contains(&n){return Err("Color outside limits");}
    Ok(n as u32)
}
fn id(value:&str)->bool{
    (1..=80).contains(&value.len()) &&
    value.bytes().all(|c|c.is_ascii_alphanumeric()||matches!(c,b'_'|b'-'))
}
fn resource(value:&str)->bool{
    let Some((namespace,path))=value.split_once(':') else{return false};
    !namespace.is_empty() && !path.is_empty() && !value.contains("..")
    && namespace.bytes().all(|b|b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b,b'_'|b'.'|b'-'))
    && path.bytes().all(|b|b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b,b'_'|b'.'|b'-'|b'/'))
}
fn array<'a>(v:&'a Value,max:usize)->Result<&'a [Value],&'static str>{
    let a=v.as_array().ok_or("Expected cinematic list")?;
    if a.len()>max {return Err("Cinematic list budget exceeded");}
    Ok(a.as_slice())
}
fn rect(v:&Value)->Result<Rect,&'static str>{
    let x=finite32(field(v,"x")?,0.0,1.0)?;
    let y=finite32(field(v,"y")?,0.0,1.0)?;
    let w=finite32(field(v,"width")?,0.00001,1.0)?;
    let h=finite32(field(v,"height")?,0.00001,1.0)?;
    if x+w>1.000001 || y+h>1.000001 {return Err("Viewport outside screen");}
    Ok(Rect{x,y,w,h})
}
fn pose(v:&Value)->Result<Pose,&'static str>{
    let p=field(v,"position")?;
    let mut pos=[0.0;3];
    for (i,k) in ["x","y","z"].iter().enumerate(){
        let n=number(field(p,k)?)?;
        if n.abs()>30_000_000.0{return Err("Camera outside world");}
        pos[i]=n;
    }
    Ok(Pose{pos,
        yaw:finite32(field(v,"yaw")?,-1.0e6,1.0e6)?,
        pitch:finite32(field(v,"pitch")?,-1.0e6,1.0e6)?,
        roll:finite32(field(v,"roll")?,-1.0e6,1.0e6)?,
        fov:finite32(field(v,"fov")?,1.001,178.999)?})
}
fn keys(v:&Value)->Result<CameraCue,&'static str>{
    let frames=array(v,22)?;
    if frames.is_empty(){return Err("No cinematic keyframes");}
    let mut result=Vec::with_capacity(frames.len());
    let mut last=-1.0;
    for f in frames {
        let t=number(field(f,"seconds")?)?;
        if !(0.0..=1.0).contains(&t)||t<=last{return Err("Cinematic keyframe order");}
        result.push(Key{seconds:t,pose:pose(field(f,"pose")?)?});
        last=t;
    }
    if result[0].seconds!=0.0{return Err("First keyframe must start at zero");}
    Ok(CameraCue{revision:0,keys:result,effects:vec![]})
}
fn effect(v:&Value)->Result<Effect,&'static str>{
    let kind=string(field(v,"kind")?)?;
    if !matches!(kind,"letterbox"|"vignette"|"fade-black"|"fade-white"|"tint"|"flash"){
        return Err("Unsupported V3 effect");
    }
    let r=if v.get("rect").is_none()||v["rect"].is_null(){Rect::full()}else{rect(&v["rect"])?};
    Ok(Effect{kind:kind.into(),amount:finite32(field(v,"amount")?,0.0,1.0)?,
        color:color(field(v,"color")?)?,rect:r})
}
fn view(v:&Value)->Result<View,&'static str>{
    let name=string(field(v,"id")?)?;
    if !id(name){return Err("Invalid view ID");}
    let fade=match v.get("fade"){
        Some(value) if !value.is_null()=>{
            let e=effect(value)?;
            if e.kind!="tint"{return Err("View fade must be tint");}
            Some(Fade{color:e.color,amount:e.amount})
        }
        _=>None,
    };
    let opacity=match v.get("opacity"){Some(a) if !a.is_null()=>finite32(a,0.0,1.0)?,_=>1.0};
    Ok(View{id:name.into(),rect:rect(field(v,"rect")?)?,
        cue:keys(field(v,"keys")?)?,opacity,fade})
}
fn uuid(s:&str)->bool{
    s.len()==36 && s.bytes().enumerate().all(|(i,b)|{
        if matches!(i,8|13|18|23){b==b'-'} else{b.is_ascii_hexdigit()}
    })
}
fn actor(v:&Value)->Result<Actor,&'static str>{
    let name=string(field(v,"id")?)?;
    if !id(name){return Err("Invalid actor ID");}
    let profile=string(field(v,"profile")?)?;
    if profile.len()>16 || !profile.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'){
        return Err("Invalid actor profile");
    }
    let entity_type=match v.get("entityType"){Some(x) if !x.is_null()=>string(x)?,_=>"minecraft:mannequin"};
    if !resource(entity_type) {return Err("Invalid actor entity type");}
    let entity_id=match v.get("entityId"){Some(x) if !x.is_null()=>{
        let id=string(x)?;if !uuid(id){return Err("Invalid actor UUID");}
        Some(id.into())
    },_=>None};
    let cue=keys(field(v,"keys")?)?;
    let scales=array(field(v,"scales")?,22)?;
    if scales.len()!=cue.keys.len(){return Err("Actor scales mismatch");}
    let scales=scales.iter().map(|v|finite32(v,0.05,16.0)).collect::<Result<Vec<_>,_>>()?;
    Ok(Actor{id:name.into(),profile:profile.into(),entity_id,
        entity_type:entity_type.into(),cue,scales})
}
fn caption(v:&Value)->Result<Caption,&'static str>{
    let name=string(field(v,"id")?)?;
    let text=string(field(v,"text")?)?;
    let font=string(field(v,"font")?)?;
    if !id(name)||text.len()>8192||text.encode_utf16().count()>2048||!resource(font){
        return Err("Invalid V3 caption");
    }
    let align=string(field(v,"align")?)?;
    if !matches!(align,"left"|"center"|"right"){return Err("Invalid caption alignment");}
    Ok(Caption{
        id:name.into(),text:text.into(),font:font.into(),
        x:finite32(field(v,"x")?,-4.0,4.0)?,
        y:finite32(field(v,"y")?,-4.0,4.0)?,
        size:finite32(field(v,"size")?,0.001,1.0)?,
        color:color(field(v,"color")?)?,
        opacity:finite32(field(v,"opacity")?,0.0,1.0)?,
        rotation:finite32(field(v,"rotation")?,-1.0e6,1.0e6)?,
        align:align.into(),
    })
}
pub fn parse(wire:&str)->Result<Scene,&'static str>{
    if wire.len()>MAX_BYTES {return Err("V3 payload too large");}
    let v:Value=serde_json::from_str(wire).map_err(|_|"Invalid V3 JSON")?;
    if field(&v,"schema")?.as_i64()!=Some(3){return Err("Unsupported cinematic schema");}
    let revision=field(&v,"revision")?.as_u64().ok_or("Bad scene revision")?;
    let name=string(field(&v,"sceneId")?)?;
    if !id(name){return Err("Bad scene ID");}
    let stop=field(&v,"stop")?.as_bool().ok_or("Bad stop flag")?;
    let views=array(field(&v,"views")?,5)?.iter().map(view).collect::<Result<Vec<_>,_>>()?;
    let actors=array(field(&v,"actors")?,16)?.iter().map(actor).collect::<Result<Vec<_>,_>>()?;
    let captions=array(field(&v,"captions")?,16)?.iter().map(caption).collect::<Result<Vec<_>,_>>()?;
    let effects=match v.get("effects"){Some(f) if !f.is_null()=>array(f,8)?
        .iter().map(effect).collect::<Result<Vec<_>,_>>()?,_=>vec![]};
    let mut unique=HashSet::new();
    if views.iter().any(|v|!unique.insert(&v.id)){return Err("Repeated V3 view ID");}
    unique.clear();
    if actors.iter().any(|a|!unique.insert(&a.id)){return Err("Repeated V3 actor ID");}
    if stop&&(!views.is_empty()||!actors.is_empty()||!captions.is_empty()||!effects.is_empty()){
        return Err("Stop frame contains content");
    }
    Ok(Scene{revision,id:name.into(),stop,views,actors,captions,effects})
}
impl Scene {
    pub fn duration(&self)->f64{
        self.views.iter().map(|v|v.cue.duration())
        .chain(self.actors.iter().map(|a|a.cue.duration()))
        .fold(0.3_f64,f64::max)
    }
    pub fn primary_camera(&self,seconds:f64)->Option<Pose>{
        self.views.first().map(|v|v.cue.sample(seconds))
    }
}
#[derive(Default)]
pub struct Session{current:Option<(Instant,Scene)>,last_revision:Option<u64>}
impl Session {
    pub fn accept(&mut self,wire:&str, now:Instant)->Result<(),&'static str>{
        let next=parse(wire)?;
        if self.last_revision.is_some_and(|r|next.revision<=r){return Err("Stale V3 revision");}
        self.last_revision=Some(next.revision);
        self.current=if next.stop{None}else{Some((now,next))};
        Ok(())
    }
    pub fn active(&mut self,now:Instant)->Option<(&Scene,f64)>{
        // Test expiration before borrowing a scene to return it; otherwise
        // clearing the owned Option conflicts with the returned reference.
        let expired=self.current.as_ref().is_some_and(|(start,scene)|
            now.saturating_duration_since(*start).as_secs_f64()>scene.duration());
        if expired {self.current=None;return None;}
        self.current.as_ref().map(|(start,scene)|
            (scene,now.saturating_duration_since(*start).as_secs_f64()))
    }
    pub fn reset(&mut self){self.current=None;self.last_revision=None;}
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn parse_java_v3_and_shortest_path_camera(){
        let data=r#"{"schema":3,"revision":42,"sceneId":"test","stop":false,
        "views":[{"id":"a","rect":{"x":0,"y":0,"width":1,"height":1},
          "keys":[{"seconds":0,"pose":{"position":{"x":1,"y":65,"z":1},"yaw":350,"pitch":0,"roll":0,"fov":70}},
                  {"seconds":1,"pose":{"position":{"x":2,"y":65,"z":1},"yaw":10,"pitch":0,"roll":0,"fov":80}}],
          "opacity":1,"fade":null}],
        "actors":[],"captions":[],"effects":[{"kind":"letterbox","amount":0.5,"color":0,
          "rect":{"x":0,"y":0,"width":1,"height":1}}]}"#;
        let scene=parse(data).unwrap();
        assert_eq!(scene.views.len(),1);
        let middle=scene.primary_camera(0.5).unwrap();
        assert!((middle.yaw-360.0).abs()<0.01);
        assert_eq!(scene.duration(),1.0);
        assert!(parse(&data.replace(r#""sceneId":"test""#,r#""sceneId":"../bad""#)).is_err());
    }
    #[test]
    fn actor_scale_matches_linear_java_sampling(){
        let key=|seconds:f64| Key {seconds,pose:Pose{
            pos:[0.0,64.0,0.0],yaw:0.0,pitch:0.0,roll:0.0,fov:70.0}};
        let actor=Actor{id:"camera_actor".into(),profile:"Player".into(),
            entity_id:None,entity_type:"minecraft:mannequin".into(),
            cue:CameraCue{revision:0,keys:vec![key(0.0),key(1.0)],effects:vec![]},
            scales:vec![1.0,2.0]};
        assert_eq!(actor.sample(0.0).1,1.0);
        assert_eq!(actor.sample(0.5).1,1.5);
        assert_eq!(actor.sample(1.0).1,2.0);
        assert_eq!(actor.sample(4.0).1,2.0);
    }
    #[test]
    fn stop_and_replay_rules(){
        let stop=r#"{"schema":3,"revision":4,"sceneId":"quit","stop":true,"views":[],"actors":[],"captions":[],"effects":[]}"#;
        let now=Instant::now();
        let mut session=Session::default();
        assert!(session.accept(stop,now).is_ok());
        assert!(session.accept(stop,now).is_err());
        assert!(session.active(now).is_none());
        session.reset();
        assert!(session.accept(stop,now).is_ok());
    }
}
