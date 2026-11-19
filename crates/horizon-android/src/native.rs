//! Android host only: all gameplay, rendering, saves and network packets come from PC's crate.
use jni::{JavaVM, objects::{JObject, JString, JValue}};
use serde::Deserialize;
use std::{path::{Path, PathBuf}, sync::OnceLock};
use winit::platform::android::activity::AndroidApp;
#[cfg(feature = "vis-vsclient")]
#[path = "../../../integrations/vsclient-horizon/api/src/lib.rs"]
mod vis_vsclient;
#[cfg(feature = "vis-vsclient")]
#[path = "../../../integrations/vsclient-horizon/activation.rs"]
mod vis_autoload;
static HOME: OnceLock<PathBuf> = OnceLock::new();
const COMMIT: &str = env!("VIS_ENGINE_COMMIT");
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Launch {
    schema:u8, owner:String, instance:String, name:String, server:String, engine_commit:String,
    // The launcher checks the selected instance's VSClient .hnmod and forwards
    // only the activation bit. Android apps cannot read another app's private
    // files, so no arbitrary native source crosses an app boundary.
    #[serde(default)] vsclient:bool,
}
fn launch(app: &AndroidApp) -> Option<Launch> {
    // AndroidApp owns the Activity and VM for the lifetime of android_main.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    env.with_local_frame(16, |env| -> jni::errors::Result<Option<Launch>> {
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        let intent = env.call_method(&activity,"getIntent","()Landroid/content/Intent;",&[])?.l()?;
        let key = JObject::from(env.new_string("vis_nexo_launch")?);
        let value = env.call_method(&intent,"getStringExtra","(Ljava/lang/String;)Ljava/lang/String;",&[JValue::Object(&key)])?.l()?;
        if value.is_null() { return Ok(None); }
        let string = JString::from(value);
        let raw:String = env.get_string(&string)?.into();
        if raw.len()>2048 {return Ok(None);}
        let request:Launch=match serde_json::from_str(&raw){Ok(v)=>v,Err(_)=>return Ok(None)};
        let valid=request.schema==1 && request.engine_commit==COMMIT && (3..=24).contains(&request.owner.len())
            && request.owner.bytes().all(|c|c.is_ascii_lowercase()||c.is_ascii_digit()||c==b'_')
            && request.instance.len()==32 && request.instance.bytes().all(|c|c.is_ascii_hexdigit())
            && !request.name.is_empty() && request.name.len()<=16 && request.name.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'_')
            && request.server.len()<=253 && request.server.bytes().all(|c|c.is_ascii_alphanumeric()||b".-:[]".contains(&c));
        Ok(valid.then_some(request))
    }).ok().flatten()
}
fn prepare(root:&Path)->std::result::Result<(),String>{
    let home=HOME.get().ok_or("Android home missing")?;
    let marker=root.join("vis-engine-commit");
    if std::fs::read_to_string(&marker).ok().as_deref()==Some(COMMIT) && crate::official_install::ready(home){ return Ok(()); }
    crate::official_install::install_with_progress(home, |_|{}).map_err(|e|e.to_string())?;
    std::fs::write(marker,COMMIT).map_err(|e|e.to_string())
}
#[unsafe(no_mangle)]
fn android_main(app:AndroidApp){
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("VIS-McRust"));
    let Some(home)=app.internal_data_path() else{return;};
    let Some(request)=launch(&app) else {log::error!("Missing or incompatible Nexo engine handoff");return;};
    let game=home.join("vis-worlds").join(&request.owner).join(&request.instance);
    if let Err(error)=std::fs::create_dir_all(&game){log::error!("Game folder: {error}");return;}
    let copy=(||->std::io::Result<()>{
        let mut data=app.asset_manager().open(c"engine-data.zip").ok_or_else(||std::io::Error::other("Missing generated engine tables"))?;
        let temp=home.join("engine-data.partial");
        std::io::copy(&mut data,&mut std::fs::File::create(&temp)?)?;
        std::fs::rename(temp,home.join("engine-data.zip"))
    })();
    if let Err(error)=copy {log::error!("Engine resources: {error}");return;}
    let resources=home.join(".local/horizon-client").join(horizon_game::VANILLA_VERSION);
    let requested_vsclient=request.vsclient;
    let mut args=vec!["--resources".into(),resources.to_string_lossy().into_owned(),"--game-dir".into(),game.to_string_lossy().into_owned(),"--home".into(),home.to_string_lossy().into_owned(),"--name".into(),request.name,"--fullscreen".into(),"--render-distance".into(),"6".into()];
    if !request.server.is_empty(){args.extend(["--server".into(),request.server]);}
    let _=HOME.set(home);
    horizon_game::android::initialize(app,args,prepare);
    horizon_official::register();
    #[cfg(feature = "vis-vsclient")]
    if requested_vsclient || vis_autoload::wants_vsclient_in(&game) {
        horizon_game::mod_api::name_mods(&["vsclient"]);
        let mods: horizon_game::mod_api::Mods =
            std::sync::Arc::from(vec![vis_vsclient::hooks()]);
        log::info!("Native VSClient {} enabled for current McRust Android instance",
            vis_autoload::VERSION);
        horizon_game::main_with(mods);
        std::process::exit(0);
    }
    horizon_game::main();
    // Winit owns one loop per process. Saving/shutdown completed before returning.
    std::process::exit(0);
}
