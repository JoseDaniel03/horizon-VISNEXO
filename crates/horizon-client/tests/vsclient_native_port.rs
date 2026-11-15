//! Compile-test the native VSClient hook adapter against Horizon's actual mod API.
//! No fake trait declarations: changes in hooks are detected by rustc.
#[path = "../../../integrations/vsclient-horizon/api/src/lib.rs"]
mod vsclient_horizon;

#[test]
fn native_mod_exports_hooks() {
    let _ = vsclient_horizon::hooks();
}
