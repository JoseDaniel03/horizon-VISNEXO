//! VSClient native for VIS McRust / Horizon 26.3.
//! Shares the Java VSClient Bridge protocol 8. Server remains authoritative.
mod core;
mod controls;
mod ui;
use core::{CameraSession, CHANNEL_CAMERA, MAX_CUE_BYTES, decode_string};
use controls::{ControlsSession, InputAction, CHANNEL_INPUT, CHANNEL_PROFILE, CHANNEL_STATE, CHANNEL_VIEW};
use horizon_client::mod_api::{
    CameraPoseEvent, ClientDisconnectingEvent, ClientPacketEvent, GuiRenderPostEvent,
    ClientPlayerNetworkLoggingInEvent, ClientTickPostEvent, ClientboundPlayPacket, ComputeFovEvent,
    CustomPayload, Hooks, Host, KeyEvent, NativeScreenKind, ServerboundPlayPacket,
    ClientStartEvent, Flow, ScreenRenderPostEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent,
};
use std::{sync::Mutex, time::Instant};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(VsClientHorizon {
        camera: Mutex::new(CameraSession::default()),
        controls: Mutex::new(ControlsSession::default()),
        zoom: Mutex::new(1.0),
    })
}
struct VsClientHorizon {
    camera: Mutex<CameraSession>,
    controls: Mutex<ControlsSession>,
    zoom: Mutex<f32>,
}
fn send_actions(host: &mut dyn Host, actions: Vec<InputAction>) {
    for action in actions {
        // Matches Fabric InputPayload and Bridge VirgolandInput exactly.
        let phase = if action.pressed { "press" } else { "release" };
        let wire = format!("{}|{}|{}|{}", core::PROTOCOL, action.sequence, action.id, phase);
        if let Ok(data) = core::encode_string(&wire) {
            let _ = host.send_packet(ServerboundPlayPacket::CustomPayload(CustomPayload {
                channel: CHANNEL_INPUT.into(), data,
            }));
        }
    }
}
fn in_world(host: &dyn Host) -> bool {
    host.client_state().is_some_and(|s| s.focused
        && s.screen == NativeScreenKind::Playing && s.world.is_some())
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
        if let Ok(mut c) = self.camera.lock() { c.reset(); }
        if let Ok(mut c) = self.controls.lock() { c.reset(); }
        if let Ok(mut z) = self.zoom.lock() { *z = 1.0; }
        // A one-use bridge ticket may be injected by the parent process.
        // No fake auth, no logging secrets, no disk persistence.
        if let Ok(ticket) = std::env::var("VIS_NEXO_CLIENT_BRIDGE_TICKET") {
            if let Ok(data) = core::auth_ticket(&ticket) {
                let _ = host.send_packet(ServerboundPlayPacket::CustomPayload(CustomPayload {
                    channel: core::CHANNEL_AUTH.into(), data,
                }));
            }
        }
        let _ = host.send_packet(ServerboundPlayPacket::CustomPayload(CustomPayload {
            channel: core::CHANNEL_HELLO.into(),
            data: core::hello(),
        }));
    }

    fn client_packet(&self, host: &mut dyn Host, event: &mut ClientPacketEvent<'_>) {
        let ClientboundPlayPacket::CustomPayload(packet) = event.packet else { return };
        let channel = packet.channel.as_str();
        let max = if channel == CHANNEL_PROFILE { 32_768 } else { MAX_CUE_BYTES };
        if !matches!(channel, CHANNEL_CAMERA | CHANNEL_STATE | CHANNEL_PROFILE | CHANNEL_VIEW | "virgoland:controls") {
            return;
        }
        let Ok(text) = decode_string(&packet.data, max) else { return };
        match channel {
            CHANNEL_CAMERA => {
                if let Ok(mut c) = self.camera.lock() {
                    let _ = c.accept(text, Instant::now());
                }
            }
            CHANNEL_STATE => {
                let actions = self.controls.lock().ok()
                    .and_then(|mut c| c.apply_state(text).ok());
                if let Some(actions) = actions { send_actions(host,actions); }
            }
            CHANNEL_PROFILE => {
                let actions = self.controls.lock().ok()
                    .and_then(|mut c| c.apply_profile(text).ok());
                if let Some(actions) = actions { send_actions(host,actions); }
            }
            "virgoland:controls" => {
                if text=="8|open" {
                    let _=host.mod_screen_open(ui::ID);
                }
            }
            CHANNEL_VIEW => {
                if let Ok(zoom) = controls::parse_view(text) {
                    if let Ok(mut current) = self.zoom.lock() { *current = zoom; }
                }
            }
            _ => {}
        }
    }

    fn key(&self, host: &mut dyn Host, event: &mut KeyEvent<'_>) {
        if event.pressed && event.key=="key.keyboard.f8" && in_world(host) {
            let _=host.mod_screen_open(ui::ID);
            return;
        }
        let playing = in_world(host);
        let actions = self.controls.lock().ok()
            .map(|mut c| c.handle_key(event.key, event.pressed, playing));
        if let Some(actions) = actions { send_actions(host, actions); }
    }

    fn client_tick_post(&self, host: &mut dyn Host, _event: &mut ClientTickPostEvent) {
        // Releasing an open menu or losing focus must never leave a remote
        // vehicle, revive or other hold action stuck down on the server.
        let playing = in_world(host);
        let actions = self.controls.lock().ok().map(|mut c| c.heartbeat(playing));
        if let Some(actions) = actions { send_actions(host, actions); }
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

    fn camera_pose(&self, _host: &dyn Host, event: &mut CameraPoseEvent<'_>) {
        let Ok(mut camera) = self.camera.lock() else { return };
        if let Some(pose) = camera.sample(Instant::now()) {
            // Camera-only. The server continues to own movement and hit tests.
            *event.position = pose.pos;
            event.yaw = pose.yaw;
            event.pitch = pose.pitch;
        }
    }

    fn compute_fov(&self, _host: &dyn Host, event: &mut ComputeFovEvent) {
        if let Ok(mut camera) = self.camera.lock() {
            if let Some(pose) = camera.sample(Instant::now()) {
                event.degrees = pose.fov;
                return;
            }
        }
        // Server VIEW_V1 zoom is visual only and never changes player speed.
        if let Ok(zoom) = self.zoom.lock() {
            event.degrees = (event.degrees / *zoom).clamp(1.0, 179.0);
        }
    }

    fn gui_render_post(&self, host: &dyn Host, event: &mut GuiRenderPostEvent<'_>) {
        // Respect menus, lost focus and world teardown. Do not paint over login.
        let Some(state) = host.client_state() else { return };
        if state.screen != NativeScreenKind::Playing || state.world.is_none() { return; }
        let (width,height) = (event.batch.width,event.batch.height);
        if width <= 0.0 || height <= 0.0 { return; }
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
    }

    fn client_disconnecting(&self, _host: &dyn Host, _event: &mut ClientDisconnectingEvent) {
        if let Ok(mut c) = self.camera.lock() { c.reset(); }
        if let Ok(mut c) = self.controls.lock() { c.reset(); }
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
