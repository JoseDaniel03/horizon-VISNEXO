//! Compile and integration tests against Horizon's real mod API / dispatch.
#[path = "../../../integrations/vsclient-horizon/api/src/lib.rs"]
mod vsclient_horizon;

use horizon_client::mod_api::{
    ClientboundPlayPacket, CustomPayload, Host, dispatch_camera_pose,
    dispatch_client_packet, dispatch_compute_fov,
};

struct TestHost;
impl Host for TestHost {}

#[test]
fn native_mod_exports_hooks() {
    let _ = vsclient_horizon::hooks();
}

#[test]
fn camera_packet_reaches_native_pose_and_fov() {
    let mods = vec![vsclient_horizon::hooks()];
    let msg = "v1|12|0,15,78,-5,87,22,0,82;4,15,78,-5,87,22,0,82|";
    assert!(msg.len() < 128);
    let mut data = vec![msg.len() as u8];
    data.extend_from_slice(msg.as_bytes());
    let packet = ClientboundPlayPacket::CustomPayload(CustomPayload {
        channel: "virgoland:camera_v1".into(),data,
    });
    dispatch_client_packet(&mut TestHost,&mods,&packet);
    let mut pos = [0.0,0.0,0.0];
    let mut yaw = 0.0;
    let mut pitch = 0.0;
    dispatch_camera_pose(&TestHost,&mods,0.0,&mut pos,&mut yaw,&mut pitch);
    assert_eq!(pos,[15.0,78.0,-5.0]);
    assert_eq!((yaw,pitch),(87.0,22.0));
    let mut fov = 70.0;
    dispatch_compute_fov(&TestHost,&mods,0.0,[0.0,0.0,0.0],&mut fov);
    assert_eq!(fov,82.0);
}
