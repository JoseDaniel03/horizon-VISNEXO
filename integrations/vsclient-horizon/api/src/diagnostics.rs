//! Private-by-default VSClient protocol diagnostics.
//! No tickets, chat, IPs or packet content are retained or exported.
//! Small bounded state can be shown with F7 and saved only by explicit F6.
use std::time::{Duration,Instant};

#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct Counters {
    pub received: u64,
    pub rejected: u64,
    pub sent: u64,
    pub send_errors: u64,
    pub camera_v1: u64,
    pub cinematic_v3: u64,
    pub profile: u64,
    pub state: u64,
    pub view: u64,
    pub controls: u64,
    pub vsrender: u64,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PacketKind {Camera,Cinematic,Profile,State,View,Controls,Render}
impl PacketKind {
    fn label(self)->&'static str {
        match self {
            Self::Camera=>"CAMERA_V1",
            Self::Cinematic=>"CINEMATIC_V3",
            Self::Profile=>"PROFILE",
            Self::State=>"STATE",
            Self::View=>"VIEW",
            Self::Controls=>"CONTROLS_UI",
            Self::Render=>"VSRENDER",
        }
    }
}
#[derive(Clone,Debug)]
pub struct Diagnostics {
    pub visible: bool,
    pub connected: bool,
    pub hello_sent: bool,
    pub state_confirmed: bool,
    pub profile_confirmed: bool,
    pub counters: Counters,
    started: Option<Instant>,
    last_event: &'static str,
}
impl Default for Diagnostics {
    fn default()->Self{Self{visible:false,connected:false,hello_sent:false,
        state_confirmed:false,profile_confirmed:false,counters:Counters::default(),
        started:None,last_event:"Sin conexion"}}
}
impl Diagnostics {
    pub fn connect(&mut self, now:Instant){
        self.connected=true;
        self.hello_sent=false;
        self.state_confirmed=false;
        self.profile_confirmed=false;
        self.counters=Counters::default();
        self.started=Some(now);
        self.last_event="Conectando: esperando Bridge";
    }
    pub fn hello(&mut self,ok:bool){
        self.hello_sent=ok;
        self.sent(ok);
        self.last_event=if ok{"HELLO enviado"}else{"Error enviando HELLO"};
    }
    pub fn sent(&mut self,ok:bool){
        if ok{self.counters.sent=self.counters.sent.saturating_add(1);}
        else{self.counters.send_errors=self.counters.send_errors.saturating_add(1);}
    }
    pub fn packet(&mut self,kind:PacketKind,accepted:bool){
        self.counters.received=self.counters.received.saturating_add(1);
        if !accepted{
            self.counters.rejected=self.counters.rejected.saturating_add(1);
            self.last_event="Paquete VSClient rechazado";
            return;
        }
        match kind{
            PacketKind::Camera=>self.counters.camera_v1+=1,
            PacketKind::Cinematic=>self.counters.cinematic_v3+=1,
            PacketKind::Profile=>{self.counters.profile+=1;self.profile_confirmed=true;}
            PacketKind::State=>{self.counters.state+=1;self.state_confirmed=true;}
            PacketKind::View=>self.counters.view+=1,
            PacketKind::Controls=>self.counters.controls+=1,
            PacketKind::Render=>self.counters.vsrender+=1,
        }
        self.last_event=kind.label();
    }
    pub fn disconnect(&mut self){
        self.connected=false;
        self.hello_sent=false;
        self.state_confirmed=false;
        self.profile_confirmed=false;
        self.started=None;
        self.last_event="Desconectado";
    }
    pub fn toggle(&mut self)->bool{self.visible=!self.visible;self.visible}
    pub fn lines(&self,now:Instant)->Vec<String>{
        let status=if !self.connected{"Sin conexion"}
            else if self.profile_confirmed && self.state_confirmed {"Bridge activo"}
            else if self.hello_sent {"HELLO enviado; esperando respuesta"}
            else {"Esperando HELLO"};
        let seconds=self.started.map_or(0,|s|now.saturating_duration_since(s).as_secs());
        let c=&self.counters;
        vec![
            format!("VSClient McRust | {status}"),
            format!("Tiempo: {}s | HELLO: {} | Perfil: {} | Estado: {}",
                seconds,self.hello_sent,self.profile_confirmed,self.state_confirmed),
            format!("RX {} | Rechazados {} | TX {} | Errores TX {}",
                c.received,c.rejected,c.sent,c.send_errors),
            format!("Camara {} | Cinematica {} | Controles {} | Vista {}",
                c.camera_v1,c.cinematic_v3,c.profile+c.controls,c.view),
            format!("VSRender {} | Ultimo: {}",c.vsrender,self.last_event),
            "F7 ocultar | F6 guardar reporte en la instancia".into(),
        ]
    }
    pub fn report(&self,now:Instant)->String{
        // Never include the hostname, account, token, raw wire, chat, or paths.
        let mut text=String::from("VSClient MCRust — diagnostico local sin datos privados\n");
        for line in self.lines(now){text.push_str(&line);text.push('\n');}
        text.truncate(1500);text
    }
    pub fn elapsed(&self,now:Instant)->Duration{
        self.started.map_or(Duration::ZERO,|s|now.saturating_duration_since(s))
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn connection_lifecycle_and_rejections_do_not_claim_handshake(){
        let now=Instant::now();
        let mut d=Diagnostics::default();
        d.connect(now);
        assert!(!d.profile_confirmed);
        d.hello(true);
        d.packet(PacketKind::State,false);
        assert!(!d.state_confirmed);
        d.packet(PacketKind::Profile,true);
        d.packet(PacketKind::State,true);
        assert!(d.lines(now)[0].contains("Bridge activo"));
        assert_eq!(d.counters.rejected,1);
        d.disconnect();
        assert!(!d.connected && !d.hello_sent && !d.profile_confirmed);
    }
    #[test]
    fn report_is_bounded_and_excludes_private_data(){
        let now=Instant::now();
        let mut d=Diagnostics::default();
        d.connect(now);
        d.hello(false);
        assert_eq!(d.counters.send_errors,1);
        let report=d.report(now);
        assert!(report.len()<=1500);
        assert!(!report.contains("VIS_NEXO_CLIENT_BRIDGE_TICKET"));
        assert!(!report.contains("host"));
        assert!(d.toggle());
    }
}
