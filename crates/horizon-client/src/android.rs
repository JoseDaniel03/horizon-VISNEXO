//! Android platform bridge. Gameplay remains the very same `horizon-client` crate as PC.
use std::{cell::RefCell, path::Path, sync::OnceLock};
use winit::platform::android::{EventLoopBuilderExtAndroid, activity::AndroidApp};
thread_local! { static APP: RefCell<Option<AndroidApp>> = const { RefCell::new(None) }; }
static ARGS: OnceLock<Vec<String>> = OnceLock::new();
type Installer = fn(&Path) -> Result<(), String>;
static INSTALLER: OnceLock<Installer> = OnceLock::new();
pub fn initialize(app: AndroidApp, args: Vec<String>, installer: Installer) {
    APP.with(|slot| *slot.borrow_mut() = Some(app));
    let _ = ARGS.set(args);
    let _ = INSTALLER.set(installer);
}
pub fn arguments() -> Vec<String> { ARGS.get().cloned().unwrap_or_default() }
pub fn event_loop() -> Result<winit::event_loop::EventLoop<()>, winit::error::EventLoopError> {
    APP.with(|slot| winit::event_loop::EventLoop::builder()
        .with_android_app(slot.borrow().as_ref().expect("Android host initializes the activity").clone()).build())
}
pub fn has_resource_installer() -> bool { INSTALLER.get().is_some() }
pub fn prepare_resources(root: &Path) -> Result<(), String> {
    INSTALLER.get().ok_or("Android resource installer missing")?(root)
}

pub use crate::skins::set_android_skin;
