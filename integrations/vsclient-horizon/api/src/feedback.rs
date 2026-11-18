//! Native, bounded camera impulses and first-person reload cues from VSRender.
//! No ammo, attack or inventory state is modified; the server owns gameplay.
use std::time::{Duration,Instant};
pub const CHANNEL_EFFECT:&str="vsrender:action_effect";
pub const CHANNEL_RELOAD:&str="vsrender:reload_arms";
#[derive(Clone,Copy,Debug,Default,PartialEq)]
pub struct Offset{pub pitch:f32,pub yaw:f32,pub fov:f32}
#[derive(Clone,Debug)]
struct Pulse{pitch:f32,yaw:f32,shake:f32,fov:f32,start:Instant,duration:Duration}
#[derive(Clone,Debug)]
struct Reload{start:Instant,duration:Duration,slot:u8}
#[derive(Default)]
pub struct Session{
    pulses:Vec<Pulse>,
    reload:Option<Reload>,
}
impl Session{
    pub fn accept_effect(&mut self,wire:&str,now:Instant)->Result<(),&'static str>{
        if wire.len()>128{return Err("Oversized action effect");}
        let cols:Vec<&str>=wire.split('|').collect();
        if cols.len()!=6 || cols[0]!="1" {return Err("Invalid action effect schema");}
        let vals:Vec<f32>=cols[1..].iter().map(|s|s.parse::<f32>()
            .map_err(|_|"Non-numeric action effect"))
            .collect::<Result<_,_>>()?;
        if vals.iter().any(|v|!v.is_finite())
            || !(0.0..=8.0).contains(&vals[0])||vals[1].abs()>4.0
            || !(0.0..=3.0).contains(&vals[2]) || vals[3].abs()>15.0
            || !(1.0..=60.0).contains(&vals[4])||vals[4].fract()!=0.0 {
            return Err("Unsafe action effect values");
        }
        if self.pulses.len()>=16{self.pulses.remove(0);}
        self.pulses.push(Pulse{pitch:vals[0],yaw:vals[1],shake:vals[2],fov:vals[3],
            start:now,duration:Duration::from_millis((vals[4] as u64)*50)});
        Ok(())
    }
    pub fn sample(&mut self,now:Instant)->Offset{
        self.pulses.retain(|p|now.saturating_duration_since(p.start)<p.duration);
        let mut o=Offset::default();
        for p in &self.pulses {
            let t=now.saturating_duration_since(p.start).as_secs_f32()/p.duration.as_secs_f32();
            let envelope=if t<0.2 {
                (t/0.2*std::f32::consts::FRAC_PI_2).sin()
            }else{((1.0-t)/0.8).max(0.0).powi(2)};
            let shake=(t*6.0*std::f32::consts::PI).sin()*p.shake*envelope;
            o.pitch+=-p.pitch*envelope+shake;
            o.yaw+=p.yaw*envelope+shake*0.6;
            o.fov+=p.fov*envelope;
        }
        o.pitch=o.pitch.clamp(-12.0,12.0);
        o.yaw=o.yaw.clamp(-8.0,8.0);
        o.fov=o.fov.clamp(-20.0,20.0);
        o
    }
    pub fn accept_reload(&mut self,raw:&[u8],now:Instant)->Result<(),&'static str>{
        let (version,ticks,slot)=decode_reload(raw)?;
        if version!=1{return Err("Invalid reload version");}
        if ticks==0 {self.reload=None;return Ok(());}
        self.reload=Some(Reload{start:now,duration:Duration::from_millis((ticks as u64)*50),
            slot:slot as u8});
        Ok(())
    }
    /// Reach, insert, return: same staged easing as Fabric ReloadArmsState.
    pub fn reload_reach(&mut self,now:Instant)->f32{
        let Some(r)=&self.reload else{return 0.0};
        let p=now.saturating_duration_since(r.start).as_secs_f32()/r.duration.as_secs_f32();
        if p>=1.0 {self.reload=None;return 0.0;}
        let smooth=|t:f32|{let t=t.clamp(0.0,1.0);t*t*(3.0-2.0*t)};
        if p<0.35 {smooth(p/0.35)}else if p<0.65{1.0}
        else{1.0-smooth((p-0.65)/0.35)}
    }
    pub fn reset(&mut self){self.pulses.clear();self.reload=None;}
    pub fn active_reload_slot(&self)->Option<u8>{self.reload.as_ref().map(|r|r.slot)}
}
fn read_varint(data:&[u8],offset:&mut usize)->Result<i32,&'static str>{
    let mut result=0_u32;
    for index in 0..5 {
        let b=*data.get(*offset).ok_or("Incomplete reload varint")?;
        *offset+=1;
        if index==4 && b&0xf0!=0 {return Err("Overflowed reload varint");}
        result|=u32::from(b&0x7f)<<(7*index);
        if b&0x80==0 {return Ok(result as i32);}
    }
    Err("Reload VarInt exceeds five bytes")
}
fn decode_reload(data:&[u8])->Result<(i32,i32,i32),&'static str>{
    if data.len()>15 {return Err("Reload packet too large");}
    let mut offset=0;
    let version=read_varint(data,&mut offset)?;
    let ticks=read_varint(data,&mut offset)?;
    let slot=read_varint(data,&mut offset)?;
    if offset!=data.len() || !(0..=72_000).contains(&ticks)||!(0..=8).contains(&slot){
        return Err("Invalid reload packet");
    }
    Ok((version,ticks,slot))
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn recoil_is_bounded_and_expires(){
        let now=Instant::now();
        let mut s=Session::default();
        assert!(s.accept_effect("1|8|4|3|15|10",now).is_ok());
        assert!(s.accept_effect("1|NaN|0|0|0|10",now).is_err());
        assert!(s.accept_effect("1|8|4|3|15|600000",now).is_err());
        let p=s.sample(now+Duration::from_millis(125));
        assert!(p.pitch.abs()<=12.0&&p.yaw.abs()<=8.0&&p.fov.abs()<=20.0);
        assert_eq!(s.sample(now+Duration::from_secs(2)),Offset::default());
    }
    #[test]
    fn reload_varints_are_exact_and_timed(){
        let now=Instant::now();
        let mut s=Session::default();
        assert!(s.accept_reload(&[1,20,4],now).is_ok());
        assert_eq!(s.active_reload_slot(),Some(4));
        assert!(s.reload_reach(now+Duration::from_millis(450))>0.9);
        assert_eq!(s.reload_reach(now+Duration::from_secs(2)),0.0);
        for data in [&[1,1][..],&[1,1,9],&[1,1,0,0],&[1,255,255,255,255,127,1]]{
            assert!(s.accept_reload(data,now).is_err());
        }
    }
}
