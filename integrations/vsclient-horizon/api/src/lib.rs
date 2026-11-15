//! VSClient native adapter for Horizon/MCRust, not a Fabric JAR.
//! Uses Horizon's public trait hooks. Gameplay remains server authoritative.
//! This first port implements protocol 8 hello and cinematic camera/FOV.
//! Missing Java-only features are intentionally NOT advertised to the server.
mod core;
use core::{CameraSession, CHANNEL_CAMERA, MAX_CUE_BYTES, decode_string};
use horizon_client::mod_api::{
    CameraPoseEvent, ClientDisconnectingEvent, ClientPacketEvent,
    ClientPlayerNetworkLoggingInEvent, ClientboundPlayPacket, ComputeFovEvent,
    CustomPayload, Hooks, Host, ServerboundPlayPacket,
};
use std::{sync::Mutex, time::Instant};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(VsClientHorizon { camera: Mutex::new(CameraSession::default()) })
}
struct VsClientHorizon {
    camera: Mutex<CameraSession>,
}
impl Hooks for VsClientHorizon {
    fn client_player_network_logging_in(
        &self,
        host: &mut dyn Host,
        _event: &mut ClientPlayerNetworkLoggingInEvent,
    ) {
        if let Ok(mut camera) = self.camera.lock() { camera.reset(); }
        // The Java Bridge understands a Minecraft VarInt length-prefixed
        // UTF-8 string, not raw UTF-8 nor Horizon's own mod-net framing.
        // We only advertise CAMERA_V1 while cinematic_v3, keys and HUD are pending.
        let _ = host.send_packet(ServerboundPlayPacket::CustomPayload(CustomPayload {
            channel: core::CHANNEL_HELLO.into(),
            data: core::hello(),
        }));
    }

    fn client_packet(&self, _host: &mut dyn Host, event: &mut ClientPacketEvent<'_>) {
        let ClientboundPlayPacket::CustomPayload(packet) = event.packet else {return;};
        if packet.channel != CHANNEL_CAMERA {return;}
        let Ok(text) = decode_string(&packet.data, MAX_CUE_BYTES) else {return;};
        if let Ok(mut camera) = self.camera.lock() {
            let _ = camera.accept(text, Instant::now());
        }
    }

    fn camera_pose(&self, _host: &dyn Host, event: &mut CameraPoseEvent<'_>) {
        let Ok(mut camera) = self.camera.lock() else {return;};
        if let Some(pose) = camera.sample(Instant::now()) {
            // Visual-only; the client's collision, aim and player location
            // stay vanilla and are validated by the server.
            *event.position = pose.pos;
            event.yaw = pose.yaw;
            event.pitch = pose.pitch;
        }
    }

    fn compute_fov(&self, _host: &dyn Host, event: &mut ComputeFovEvent<'_>) {
        let Ok(mut camera) = self.camera.lock() else {return;};
        if let Some(pose) = camera.sample(Instant::now()) {
            event.degrees = pose.fov;
        }
    }

    fn client_disconnecting(&self, _host: &dyn Host, _event: &mut ClientDisconnectingEvent) {
        if let Ok(mut camera) = self.camera.lock() {camera.reset();}
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
