//! Android host only: all gameplay, rendering, saves and network packets come from PC's crate.
use base64::Engine;
use jni::{
    JavaVM,
    objects::{JObject, JString, JValue},
};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};
use winit::platform::android::activity::AndroidApp;
static HOME: OnceLock<PathBuf> = OnceLock::new();
const COMMIT: &str = env!("VIS_ENGINE_COMMIT");
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Launch {
    schema: u8,
    owner: String,
    instance: String,
    name: String,
    server: String,
    engine_commit: String,
}
fn launch(app: &AndroidApp) -> Option<Launch> {
    // AndroidApp owns the Activity and VM for the lifetime of android_main.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    env.with_local_frame(16, |env| -> jni::errors::Result<Option<Launch>> {
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        let intent = env
            .call_method(&activity, "getIntent", "()Landroid/content/Intent;", &[])?
            .l()?;
        let key = JObject::from(env.new_string("vis_nexo_launch")?);
        let value = env
            .call_method(
                &intent,
                "getStringExtra",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[JValue::Object(&key)],
            )?
            .l()?;
        if value.is_null() {
            return Ok(None);
        }
        let string = JString::from(value);
        let raw: String = env.get_string(&string)?.into();
        if raw.len() > 2048 {
            return Ok(None);
        }
        let request: Launch = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        let valid = request.schema == 1
            && request.engine_commit == COMMIT
            && (3..=24).contains(&request.owner.len())
            && request
                .owner
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
            && request.instance.len() == 32
            && request.instance.bytes().all(|c| c.is_ascii_hexdigit())
            && !request.name.is_empty()
            && request.name.len() <= 16
            && request
                .name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
            && request.server.len() <= 253
            && request
                .server
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".-:[]".contains(&c));
        Ok(valid.then_some(request))
    })
    .ok()
    .flatten()
}
/// Optional public VIS skin (no credentials) from Nexo Android. The actual
/// identity handoff remains schema-1 and independently validated.
fn launcher_skin(app: &AndroidApp) -> Option<Vec<u8>> {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    env.with_local_frame(12, |env| -> jni::errors::Result<Option<Vec<u8>>> {
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        let intent = env
            .call_method(&activity, "getIntent", "()Landroid/content/Intent;", &[])?
            .l()?;
        let key = JObject::from(env.new_string("vis_nexo_skin")?);
        let value = env
            .call_method(
                &intent,
                "getStringExtra",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[JValue::Object(&key)],
            )?
            .l()?;
        if value.is_null() {
            return Ok(None);
        }
        let text: String = env.get_string(&JString::from(value))?.into();
        if text.len() > 87500 || !text.starts_with("data:image/png;base64,") {
            return Ok(None);
        }
        let raw = &text["data:image/png;base64,".len()..];
        let decoded = match base64::engine::general_purpose::STANDARD.decode(raw) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        if decoded.len() < 33
            || decoded.len() > 65536
            || !decoded.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10])
            || &decoded[12..16] != b"IHDR"
        {
            return Ok(None);
        }
        Ok(Some(decoded))
    })
    .ok()
    .flatten()
}
// Java host serializes management with the native game's lifetime.
fn game_directory(app: &AndroidApp, owner: &str, instance: &str) -> Option<PathBuf> {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let result = env.with_local_frame(16, |env| -> jni::errors::Result<PathBuf> {
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        let loader = env
            .call_method(
                &activity,
                "getClassLoader",
                "()Ljava/lang/ClassLoader;",
                &[],
            )?
            .l()?;
        let name = JObject::from(env.new_string("dev.horizon.android.VisGameStore")?);
        let class = jni::objects::JClass::from(
            env.call_method(
                loader,
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&name)],
            )?
            .l()?,
        );
        let owner = JObject::from(env.new_string(owner)?);
        let instance = JObject::from(env.new_string(instance)?);
        let value = env
            .call_static_method(
                class,
                "begin",
                "(Landroid/app/Activity;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
                &[
                    JValue::Object(&activity),
                    JValue::Object(&owner),
                    JValue::Object(&instance),
                ],
            )?
            .l()?;
        let text: String = env.get_string(&JString::from(value))?.into();
        Ok(PathBuf::from(text))
    });
    let _ = env.exception_clear();
    result.ok()
}
struct GameLease(AndroidApp);
impl Drop for GameLease {
    fn drop(&mut self) {
        if let Ok(vm) = unsafe { JavaVM::from_raw(self.0.vm_as_ptr().cast()) } {
            if let Ok(mut env) = vm.attach_current_thread() {
                let result = (|| -> jni::errors::Result<()> {
                    let activity = unsafe { JObject::from_raw(self.0.activity_as_ptr().cast()) };
                    let loader = env
                        .call_method(
                            &activity,
                            "getClassLoader",
                            "()Ljava/lang/ClassLoader;",
                            &[],
                        )?
                        .l()?;
                    let name = JObject::from(env.new_string("dev.horizon.android.VisGameStore")?);
                    let class = jni::objects::JClass::from(
                        env.call_method(
                            loader,
                            "loadClass",
                            "(Ljava/lang/String;)Ljava/lang/Class;",
                            &[JValue::Object(&name)],
                        )?
                        .l()?,
                    );
                    env.call_static_method(class, "end", "()V", &[])?;
                    Ok(())
                })();
                let _ = result;
                let _ = env.exception_clear();
            }
        }
    }
}
fn prepare(root: &Path) -> std::result::Result<(), String> {
    let home = HOME.get().ok_or("Android home missing")?;
    let marker = root.join("vis-engine-commit");
    if std::fs::read_to_string(&marker).ok().as_deref() == Some(COMMIT)
        && crate::official_install::ready(home)
    {
        return Ok(());
    }
    crate::official_install::install_with_progress(home, |_| {}).map_err(|e| e.to_string())?;
    std::fs::write(marker, COMMIT).map_err(|e| e.to_string())
}
// Every rejected handoff/bootstrap returns to Nexo instead of leaving a black activity.
struct ActivityFinish(AndroidApp);
impl Drop for ActivityFinish {
    fn drop(&mut self) {
        if let Ok(vm) = unsafe { JavaVM::from_raw(self.0.vm_as_ptr().cast()) } {
            if let Ok(mut env) = vm.attach_current_thread() {
                let activity = unsafe { JObject::from_raw(self.0.activity_as_ptr().cast()) };
                let _ = env.call_method(&activity, "finish", "()V", &[]);
                let _ = env.exception_clear();
            }
        }
    }
}
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    let _finish = ActivityFinish(app.clone());
    let Some(home) = app.internal_data_path() else {
        return;
    };
    horizon_game::android::initialize_logging(&home);
    let Some(request) = launch(&app) else {
        log::error!("Missing or incompatible Nexo engine handoff");
        return;
    };
    // Load public VIS cosmetic into Horizon's existing offline-player skin
    // lookup before any game state starts. Signed premium textures take priority.
    if let Some(skin) = launcher_skin(&app) {
        if let Err(reason) = horizon_game::install_public_offline_skin(&skin, "classic") {
            log::warn!("Rejected optional VIS skin: {reason}");
        }
    }
    let Some(game) = game_directory(&app, &request.owner, &request.instance) else {
        log::error!("Instance storage unavailable or game already running");
        return;
    };
    let _lease = GameLease(app.clone());
    let copy = (|| -> std::io::Result<()> {
        let mut data = app
            .asset_manager()
            .open(c"engine-data.zip")
            .ok_or_else(|| std::io::Error::other("Missing generated engine tables"))?;
        let temp = home.join("engine-data.partial");
        std::io::copy(&mut data, &mut std::fs::File::create(&temp)?)?;
        std::fs::rename(temp, home.join("engine-data.zip"))
    })();
    if let Err(error) = copy {
        log::error!("Engine resources: {error}");
        return;
    }
    let resources = home
        .join(".local/horizon-client")
        .join(horizon_platform::version::VANILLA_VERSION);
    let mut args = vec![
        "--resources".into(),
        resources.to_string_lossy().into_owned(),
        "--game-dir".into(),
        game.to_string_lossy().into_owned(),
        "--home".into(),
        home.to_string_lossy().into_owned(),
        "--name".into(),
        request.name,
        "--fullscreen".into(),
    ];
    if !request.server.is_empty() {
        args.extend(["--server".into(), request.server]);
    }
    let _ = HOME.set(home);
    crate::health::initialize(app.clone());
    horizon_game::android::health::initialize(crate::health::sample);
    horizon_game::android::initialize(app, args, prepare);
    horizon_game::main();
    // Winit owns one loop per process. Saving/shutdown completed before returning.
    std::process::exit(0);
}
