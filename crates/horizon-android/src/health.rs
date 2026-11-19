//! Native Android sampling lives in the host: the gameplay crate forbids unsafe code.
use horizon_game::android::health::Reading;
use jni::{
    JavaVM,
    objects::{JObject, JValue},
};
use std::sync::OnceLock;
use winit::platform::android::activity::AndroidApp;
static APP: OnceLock<AndroidApp> = OnceLock::new();
pub fn initialize(app: AndroidApp) {
    let _ = APP.set(app);
}
fn with_env<T>(
    f: impl FnOnce(&mut jni::JNIEnv<'_>, &JObject<'_>) -> jni::errors::Result<T>,
) -> Option<T> {
    (|| {
        let app = APP.get()?;
        // AndroidApp retains the VM/activity for the entire event-loop lifetime.
        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let result = env.with_local_frame(24, |env| {
            let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
            f(env, &activity)
        });
        if result.is_err() {
            let _ = env.exception_clear();
        }
        result.ok()
    })()
}
fn service<'a>(
    env: &mut jni::JNIEnv<'a>,
    activity: &JObject<'_>,
    name: &str,
) -> jni::errors::Result<JObject<'a>> {
    let key = JObject::from(env.new_string(name)?);
    env.call_method(
        activity,
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[JValue::Object(&key)],
    )?
    .l()
}
pub fn sample() -> Reading {
    let memory = with_env(|env, activity| {
        let manager = service(env, activity, "activity")?;
        let info = env.new_object("android/app/ActivityManager$MemoryInfo", "()V", &[])?;
        env.call_method(
            &manager,
            "getMemoryInfo",
            "(Landroid/app/ActivityManager$MemoryInfo;)V",
            &[JValue::Object(&info)],
        )?;
        let bytes = env.get_field(&info, "totalMem", "J")?.j()?.max(0) as u64;
        let low = env
            .call_method(&manager, "isLowRamDevice", "()Z", &[])?
            .z()?;
        Ok((bytes, low))
    })
    .unwrap_or((0, true));
    let thermal = with_env(|env, activity| {
        let api = env
            .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
            .i()?;
        if api < 29 {
            return Ok(None);
        }
        let manager = service(env, activity, "power")?;
        Ok(Some(
            env.call_method(&manager, "getCurrentThermalStatus", "()I", &[])?
                .i()?,
        ))
    })
    .flatten();
    let trim = with_env(|env, _| {
        let info = env.new_object(
            "android/app/ActivityManager$RunningAppProcessInfo",
            "()V",
            &[],
        )?;
        env.call_static_method(
            "android/app/ActivityManager",
            "getMyMemoryState",
            "(Landroid/app/ActivityManager$RunningAppProcessInfo;)V",
            &[JValue::Object(&info)],
        )?;
        env.get_field(&info, "lastTrimLevel", "I")?.i()
    })
    .unwrap_or(0);
    Reading {
        memory_bytes: memory.0,
        low_ram: memory.1,
        thermal,
        trim,
    }
}
