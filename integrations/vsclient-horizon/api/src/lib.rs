//! VSClient native for VIS McRust / Horizon 26.3.
//! Shares the Java VSClient Bridge protocol 8. Server remains authoritative.
mod core;
mod cinematic;
mod feedback;
mod controls;
mod ui;
mod diagnostics;
use core::{CameraSession, CHANNEL_CAMERA, MAX_CUE_BYTES, decode_string};
use diagnostics::{Diagnostics,PacketKind};
use controls::{ControlsSession, InputAction, CHANNEL_INPUT, CHANNEL_PROFILE, CHANNEL_STATE, CHANNEL_VIEW};
use horizon_client::mod_api::{
    CameraPoseEvent, ClientDisconnectingEvent, ClientPacketEvent, GuiRenderPostEvent,
    ClientPlayerNetworkLoggingInEvent, ClientTickPostEvent, ClientboundPlayPacket, ComputeFovEvent,
    CustomPayload, Hooks, Host, KeyEvent, NativeScreenKind, ServerboundPlayPacket,
    ClientStartEvent, Flow, ScreenRenderPostEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent, MouseButtonPreEvent, MovementInputEvent,
    CameraShakeEvent, RenderHandPoseEvent, Hand,
};
use std::{sync::Mutex, time::Instant};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(VsClientHorizon {
        camera: Mutex::new(CameraSession::default()),
        cinematic: Mutex::new(cinematic::Session::default()),
        feedback: Mutex::new(feedback::Session::default()),
        controls: Mutex::new(ControlsSession::default()),
        zoom: Mutex::new(1.0),
        diagnostics: Mutex::new(Diagnostics::default()),
    })
}
struct VsClientHorizon {
    camera: Mutex<CameraSession>,
    cinematic: Mutex<cinematic::Session>,
    feedback: Mutex<feedback::Session>,
    controls: Mutex<ControlsSession>,
    zoom: Mutex<f32>,
    diagnostics: Mutex<Diagnostics>,
}
fn send_actions(host: &mut dyn Host, actions: Vec<InputAction>, diagnostics: &Mutex<Diagnostics>) {
    for action in actions {
        // Matches Fabric InputPayload and Bridge VirgolandInput exactly.
        let phase = if action.pressed { "press" } else { "release" };
        let wire = format!("{}|{}|{}|{}", core::PROTOCOL, action.sequence, action.id, phase);
        if let Ok(data) = core::encode_string(&wire) {
            let ok = host.send_packet(ServerboundPlayPacket::CustomPayload(CustomPayload {
                channel: CHANNEL_INPUT.into(), data,
            })).is_ok();
            if let Ok(mut diag)=diagnostics.lock(){diag.sent(ok);}
        }
    }
}
fn in_world(host: &dyn Host) -> bool {
    host.client_state().is_some_and(|s| s.focused
        && s.screen == NativeScreenKind::Playing && s.world.is_some())
}
/// V3 overlays use Horizon's safe GUI primitives. No direct renderer access:
/// additional world views, rotated/scaled text and actor geometry need a GPU
/// implementation before we can advertise the full V3 capability.
fn paint_scene(host:&dyn Host,batch:&mut horizon_client::mod_api::HudBatch,
    scene:&cinematic::Scene,width:f32,height:f32)
{
    for e in &scene.effects {
        let alpha=(e.amount.clamp(0.0,1.0)*255.0).round() as u32;
        if alpha==0 {continue;}
        let r=[e.rect.x*width,e.rect.y*height,e.rect.w*width,e.rect.h*height];
        match e.kind.as_str() {
            "letterbox" => {
                let h=(height*0.15*e.amount).min(height*0.2);
                host.fill(batch,[0.0,0.0,width,h],0xff000000);
                host.fill(batch,[0.0,height-h,width,h],0xff000000);
            }
            "vignette" => {
                // Bounded, non-shader approximation; a single pass of 8 bands.
                for i in 0..8_u32 {
                    let inset=i as f32*width.min(height)*0.018;
                    let opacity=((alpha as f32)*(1.0-i as f32/8.0)/10.0) as u32;
                    let color=opacity<<24;
                    let thick=(height*0.017).max(1.0);
                    host.fill(batch,[inset,inset,width-2.0*inset,thick],color);
                    host.fill(batch,[inset,height-inset-thick,width-2.0*inset,thick],color);
                }
            }
            _ => {
                let rgb=match e.kind.as_str() {
                    "fade-black"=>0,"fade-white"|"flash"=>0xffffff,_=>e.color&0xffffff
                };
                host.fill(batch,r,(alpha<<24)|rgb);
            }
        }
    }
    for c in &scene.captions {
        if c.opacity<=0.0 {continue;}
        // Text uses Horizon's current resource-pack glyph atlas. For now the
        // native API exposes unscaled opaque text, so use the original color
        // and horizontal anchoring without claiming Java's full typography.
        let line=c.text.lines().next().unwrap_or("");
        let text:String=line.chars().take(200).collect();
        let advance=host.text_width(&text);
        let x=c.x*width - match c.align.as_str(){
            "center"=>advance*0.5,"right"=>advance,_=>0.0
        };
        let y=c.y*height;
        host.text(batch,&text,[x,y],c.color&0xffffff,true);
    }
}

impl VsClientHorizon {
    fn record(&self, kind:PacketKind, ok:bool){
        if let Ok(mut diag)=self.diagnostics.lock(){diag.packet(kind,ok);}
    }
}
impl Hooks for VsClientHorizon {
    fn client_start(&self, host: &mut dyn Host, _event: &mut ClientStartEvent) {
        if let Some(saved)=host.setting(ui::ID, "bindings") {
            if let Ok(mut controls)=self.controls.lock() {
                let _=controls.load_bindings(&saved);
            }
        }
    }
    fn client_player_network_logging_in(
        &self,
        host: &mut dyn Host,
        _event: &mut ClientPlayerNetworkLoggingInEvent,
    ) {
        if let Ok(mut diag)=self.diagnostics.lock(){diag.connect(Instant::now());}
        if let Ok(mut c) = self.camera.lock() { c.reset(); }
        if let Ok(mut c) = self.cinematic.lock() { c.reset(); }
        if let Ok(mut c) = self.controls.lock() { c.reset(); }
        if let Ok(mut z) = self.zoom.lock() { *z = 1.0; }
        if let Ok(mut f) = self.feedback.lock() { f.reset(); }
        // A one-use bridge ticket may be injected by the parent process.
        // No fake auth, no logging secrets, no disk persistence.
        if let Ok(ticket) = std::env::var("VIS_NEXO_CLIENT_BRIDGE_TICKET") {
            if let Ok(data) = core::auth_ticket(&ticket) {
                let sent=host.send_packet(ServerboundPlayPacket::CustomPayload(CustomPayload {
                    channel: core::CHANNEL_AUTH.into(), data,
                })).is_ok();
                if let Ok(mut diag)=self.diagnostics.lock(){diag.sent(sent);}
            }
        }
        let ok=host.send_packet(ServerboundPlayPacket::CustomPayload(CustomPayload {
            channel: core::CHANNEL_HELLO.into(),
            data: core::hello(),
        })).is_ok();
        if let Ok(mut diag)=self.diagnostics.lock(){diag.hello(ok);}
    }

    fn client_packet(&self, host: &mut dyn Host, event: &mut ClientPacketEvent<'_>) {
        let ClientboundPlayPacket::CustomPayload(packet) = event.packet else { return };
        let channel = packet.channel.as_str();
        if channel == feedback::CHANNEL_EFFECT {
            let ok=decode_string(&packet.data,128).ok().and_then(|text|
                self.feedback.lock().ok().map(|mut f|f.accept_effect(text,Instant::now()).is_ok())
            ).unwrap_or(false);
            self.record(PacketKind::Render,ok);
            return;
        }
        if channel == feedback::CHANNEL_RELOAD {
            let ok=self.feedback.lock().map(|mut f|
                f.accept_reload(&packet.data,Instant::now()).is_ok()).unwrap_or(false);
            self.record(PacketKind::Render,ok);
            return;
        }
        let max = if channel == CHANNEL_PROFILE || channel == cinematic::CHANNEL { 30_000 } else { MAX_CUE_BYTES };
        if !matches!(channel, CHANNEL_CAMERA | CHANNEL_STATE | CHANNEL_PROFILE | CHANNEL_VIEW | "virgoland:controls" | cinematic::CHANNEL) {
            return;
        }
        let kind=match channel{
            CHANNEL_CAMERA=>PacketKind::Camera,
            CHANNEL_STATE=>PacketKind::State,
            CHANNEL_PROFILE=>PacketKind::Profile,
            CHANNEL_VIEW=>PacketKind::View,
            cinematic::CHANNEL=>PacketKind::Cinematic,
            _=>PacketKind::Controls,
        };
        let Ok(text) = decode_string(&packet.data, max) else {
            self.record(kind,false);
            return;
        };
        match channel {
            cinematic::CHANNEL => {
                let accepted=self.cinematic.lock().is_ok_and(|mut c|c.accept(text,Instant::now()).is_ok());
                self.record(PacketKind::Cinematic,accepted);
                if accepted {
                    if let Ok(mut camera)=self.camera.lock(){camera.reset();}
                }
            }
            CHANNEL_CAMERA => {
                let accepted=self.camera.lock().map(|mut c|
                    c.accept(text,Instant::now()).is_ok()).unwrap_or(false);
                self.record(PacketKind::Camera,accepted);
            }
            CHANNEL_STATE => {
                let actions = self.controls.lock().ok()
                    .and_then(|mut c| c.apply_state(text).ok());
                let accepted=actions.is_some();
                if let Some(actions) = actions { send_actions(host,actions,&self.diagnostics); }
                self.record(kind,accepted);
            }
            CHANNEL_PROFILE => {
                let actions = self.controls.lock().ok()
                    .and_then(|mut c| c.apply_profile(text).ok());
                if let Some(actions) = actions { send_actions(host,actions,&self.diagnostics); }
            }
            "virgoland:controls" => {
                let accepted=text=="8|open" && host.mod_screen_open(ui::ID);
                self.record(PacketKind::Controls,accepted);
            }
            CHANNEL_VIEW => {
                let accepted=controls::parse_view(text).ok().and_then(|zoom|
                    self.zoom.lock().ok().map(|mut current|{*current=zoom;})).is_some();
                self.record(PacketKind::View,accepted);
            }
            _ => {}
        }
    }

    fn key(&self, host: &mut dyn Host, event: &mut KeyEvent<'_>) {
        if event.pressed && in_world(host) && event.key=="key.keyboard.f7" {
            if let Ok(mut d)=self.diagnostics.lock(){d.toggle();}
            return;
        }
        if event.pressed && in_world(host) && event.key=="key.keyboard.f6" {
            if let Ok(d)=self.diagnostics.lock() {
                if d.visible {
                    host.set_setting(ui::ID,"last_diagnostic",&d.report(Instant::now()));
                }
            }
            return;
        }
        if event.pressed && event.key=="key.keyboard.f8" && in_world(host) {
            let _=host.mod_screen_open(ui::ID);
            return;
        }
        let playing = in_world(host);
        let actions = self.controls.lock().ok()
            .map(|mut c| c.handle_key(event.key, event.pressed, playing));
        if let Some(actions) = actions { send_actions(host, actions, &self.diagnostics); }
    }

    fn client_tick_post(&self, host: &mut dyn Host, _event: &mut ClientTickPostEvent) {
        // Releasing an open menu or losing focus must never leave a remote
        // vehicle, revive or other hold action stuck down on the server.
        let playing = in_world(host);
        if !playing {
            if let Ok(mut f)=self.feedback.lock(){f.reset();}
        }
        let actions = self.controls.lock().ok().map(|mut c| c.heartbeat(playing));
        if let Some(actions) = actions { send_actions(host, actions, &self.diagnostics); }
    }

    fn mouse_button_pre(&self, host: &mut dyn Host, event: &mut MouseButtonPreEvent) -> Flow {
        // Native mouse pre-events are cancellable. If a live server profile
        // claims this button, send the semantic action BEFORE vanilla mining,
        // attacking or using an item can consume it.
        if !in_world(host) {return Flow::Continue;}
        let (claimed,actions)=self.controls.lock().map(|mut c|{
            let claimed=c.claimed_mouse(event.button);
            let key=match event.button {
                0=>"key.mouse.left",1=>"key.mouse.right",2=>"key.mouse.middle",_=>""
            };
            let actions=if claimed {c.handle_key(key,event.pressed,true)} else {vec![]};
            (claimed,actions)
        }).unwrap_or((false,vec![]));
        if claimed {
            send_actions(host, actions, &self.diagnostics);
            Flow::Cancel
        } else {Flow::Continue}
    }

    fn movement_input(&self, _host: &dyn Host, event: &mut MovementInputEvent) {
        if let Ok(c)=self.controls.lock(){
            if c.claims_move(&[87,83]) {event.forward=0.0;}
            if c.claims_move(&[65,68]) {event.strafe=0.0;}
            if c.claims_move(&[32]) {event.jump=false;}
            if c.claims_move(&[340,344]) {event.sneak=false;}
        }
    }

    fn screen_render_post(&self, host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
        if event.screen.id != "mod" || host.mod_screen() != Some(ui::ID) {return;}
        if let Ok(controls)=self.controls.lock() {
            ui::draw(host, event, &controls);
        }
    }

    fn screen_mouse_button_pressed_pre(&self, host: &dyn Host, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
        if event.screen.id!="mod" || host.mod_screen()!=Some(ui::ID) {return Flow::Continue;}
        if let Ok(mut controls)=self.controls.lock() {
            match ui::click(event.screen,&mut controls,event.button) {
                ui::Click::None => Flow::Continue,
                ui::Click::Handled => Flow::Cancel,
                ui::Click::Save => {
                    host.set_setting(ui::ID,"bindings",&controls.bindings_wire());
                    Flow::Cancel
                }
            }
        } else {Flow::Continue}
    }

    fn screen_key_pressed_pre(&self, host: &dyn Host, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
        if event.screen.id!="mod" || host.mod_screen()!=Some(ui::ID) || event.repeat {
            return Flow::Continue;
        }
        if let Ok(mut controls)=self.controls.lock() {
            if controls.capture_id().is_some() {
                let result=controls.apply_capture(event.key);
                if result == Ok(true) {
                    host.set_setting(ui::ID,"bindings",&controls.bindings_wire());
                }
                return Flow::Cancel;
            }
        }
        Flow::Continue
    }

    fn camera_shake(&self, host: &dyn Host, event: &mut CameraShakeEvent<'_>) {
        if !in_world(host) {return;}
        // Scene cameras exclude weapon recoil, matching Fabric behaviour.
        if self.cinematic.lock().ok()
            .is_some_and(|mut c|c.active(Instant::now()).is_some_and(|(s,_)|s.primary_camera(0.0).is_some())) {
            return;
        }
        if let Ok(mut f)=self.feedback.lock(){
            let recoil=f.sample(Instant::now());
            event.shake.pitch+=recoil.pitch;
            event.shake.yaw+=recoil.yaw;
        }
    }

    fn render_hand_pose(&self, host:&dyn Host, event:&mut RenderHandPoseEvent<'_>) {
        if !in_world(host) || event.hand!=Hand::Main {return;}
        if let Ok(mut f)=self.feedback.lock() {
            let reach=f.reload_reach(Instant::now());
            event.x+=0.18*reach;
            event.y-=0.22*reach;
            event.z-=0.12*reach;
            event.roll-=25.0*reach;
        }
    }

    fn camera_pose(&self, _host: &dyn Host, event: &mut CameraPoseEvent<'_>) {
        if let Ok(mut cinema)=self.cinematic.lock() {
            if let Some((scene,time))=cinema.active(Instant::now()) {
                if let Some(pose)=scene.primary_camera(time) {
                    *event.position=pose.pos;
                    event.yaw=pose.yaw;
                    event.pitch=pose.pitch;
                    return;
                }
            }
        }
        let Ok(mut camera) = self.camera.lock() else { return };
        if let Some(pose) = camera.sample(Instant::now()) {
            // Camera-only. The server continues to own movement and hit tests.
            *event.position = pose.pos;
            event.yaw = pose.yaw;
            event.pitch = pose.pitch;
        }
    }

    fn compute_fov(&self, _host: &dyn Host, event: &mut ComputeFovEvent) {
        if let Ok(mut cinema)=self.cinematic.lock() {
            if let Some((scene,time))=cinema.active(Instant::now()) {
                if let Some(pose)=scene.primary_camera(time) {
                    event.degrees=pose.fov;
                    return;
                }
            }
        }
        if let Ok(mut camera) = self.camera.lock() {
            if let Some(pose) = camera.sample(Instant::now()) {
                event.degrees = pose.fov;
                return;
            }
        }
        // Server VIEW_V1 zoom is visual only and never changes player speed.
        let recoil=self.feedback.lock().map(|mut f|f.sample(Instant::now()).fov).unwrap_or(0.0);
        if let Ok(zoom) = self.zoom.lock() {
            event.degrees = (event.degrees / *zoom + recoil).clamp(15.0, 150.0);
        }
    }

    fn gui_render_post(&self, host: &dyn Host, event: &mut GuiRenderPostEvent<'_>) {
        // Respect menus, lost focus and world teardown. Do not paint over login.
        let Some(state) = host.client_state() else { return };
        if state.screen != NativeScreenKind::Playing || state.world.is_none() { return; }
        let (width,height) = (event.batch.width,event.batch.height);
        if width <= 0.0 || height <= 0.0 { return; }
        // Render the bounded V3 effects/captions even for clients whose
        // server sends scenes without relying on capability negotiation.
        if let Ok(mut cinema)=self.cinematic.lock(){
            if let Some((scene,_time))=cinema.active(Instant::now()){
                paint_scene(host,event.batch,scene,width,height);
            }
        }
        let Ok(mut camera) = self.camera.lock() else { return };
        for overlay in camera.overlays(Instant::now()) {
            let alpha = (overlay.alpha.clamp(0.0,1.0) * 255.0).round() as u32;
            if alpha == 0 { continue; }
            let color = (alpha << 24) | overlay.rgb;
            if overlay.letterbox {
                let bar = (height * 0.12).round();
                host.fill(event.batch,[0.0,0.0,width,bar],color);
                host.fill(event.batch,[0.0,height-bar,width,bar],color);
            } else {
                host.fill(event.batch,[0.0,0.0,width,height],color);
            }
        }
        if let Ok(d)=self.diagnostics.lock() {
            if d.visible {
                let lines=d.lines(Instant::now());
                let box_height=18.0+lines.len() as f32*12.0;
                host.fill(event.batch,[8.0,8.0,width.min(510.0)-16.0,box_height],0xe4182031);
                for (i,line) in lines.iter().enumerate() {
                    host.text(event.batch,line,[16.0,17.0+i as f32*12.0],
                        if i==0 {0xffaeebba}else{0xffd8e2ef},true);
                }
            }
        }
    }

    fn client_disconnecting(&self, _host: &dyn Host, _event: &mut ClientDisconnectingEvent) {
        if let Ok(mut diag)=self.diagnostics.lock(){diag.disconnect();}
        if let Ok(mut c) = self.camera.lock() { c.reset(); }
        if let Ok(mut c) = self.cinematic.lock() { c.reset(); }
        if let Ok(mut c) = self.controls.lock() { c.reset(); }
        if let Ok(mut f) = self.feedback.lock() { f.reset(); }
        if let Ok(mut z) = self.zoom.lock() { *z = 1.0; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_entrypoint_exposes_a_hooks_object() {
        let _: Box<dyn Hooks> = hooks();
    }
}
