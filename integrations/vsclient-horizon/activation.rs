//! VSClient is statically linked into VIS Horizon releases, and only enabled
//! for a portable instance when that instance contains a compatible .hnmod.
//! Never compiles, executes or loads the mod's untrusted ZIP source at runtime.
use std::{ffi::OsString,fs,path::PathBuf};

pub const VERSION:&str="0.4.1";

fn home(args:impl IntoIterator<Item=OsString>)->Option<PathBuf>{
    let mut args=args.into_iter();
    let mut game_dir=None;
    let mut saw_game_dir=false;
    let mut fallback_home=None;
    while let Some(arg)=args.next(){
        // A shared launcher home can contain many instances; the selected
        // game directory must always take precedence regardless of CLI order.
        if arg=="--game-dir" {
            saw_game_dir=true;
            game_dir=args.next().map(PathBuf::from).filter(|p|p.is_dir());
        } else if arg=="--home" {
            fallback_home=args.next().map(PathBuf::from).filter(|p|p.is_dir());
        }
    }
    // Do not fall back to the shared launcher home if the requested
    // instance path is missing: that could enable a different profile's mod.
    if saw_game_dir { game_dir } else { fallback_home }
}
pub fn wants_vsclient()->bool{
    let Some(dir)=home(std::env::args_os()) else{return false};
    wants_vsclient_in(&dir)
}
/// Android receives the instance path from the verified native Activity
/// handoff, not from process arguments. Both platforms use the same check.
pub fn wants_vsclient_in(dir:&std::path::Path)->bool {
    let mod_file=dir.join("horizon-native-mods/vsclient.hnmod");
    let Ok(meta)=fs::symlink_metadata(&mod_file) else{return false};
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len()>64*1024*1024 {
        return false;
    }
    let enabled=fs::read_to_string(dir.join("horizon-mods.txt")).ok()
        .is_some_and(|s|s.lines().filter_map(|line|line.split_once(':'))
            .any(|(k,v)|k.trim()=="mod.vsclient"&&v.trim()=="true"));
    if !enabled{return false;}
    match horizon_mods::index::native::Bundle::read(&mod_file) {
        Ok(bundle) if bundle.manifest.id=="vsclient"
            && bundle.manifest.version==VERSION
            && bundle.manifest.game=="26.3"
            && bundle.manifest.hooks.is_none() => true,
        _=>{
            eprintln!(
                "[VIS] VSClient .hnmod invalid or incompatible with the precompiled {} module.",
                VERSION
            );
            false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn home_reads_usable_game_directory_not_display_name(){
        let temp=std::env::temp_dir();
        let args=vec![
            OsString::from("horizon-client"),OsString::from("--resources"),
            OsString::from("assets"),OsString::from("--game-dir"),
            temp.clone().into_os_string()
        ];
        assert_eq!(home(args),Some(temp.clone()));
        assert_eq!(home([OsString::from("--game-dir")]),None);
        let other=std::env::current_dir().unwrap();
        let args=vec![
            OsString::from("horizon-official"),
            OsString::from("--home"),other.clone().into_os_string(),
            OsString::from("--game-dir"),temp.clone().into_os_string(),
        ];
        assert_eq!(home(args),Some(temp.clone()));
        let args=vec![
            OsString::from("--game-dir"),temp.clone().into_os_string(),
            OsString::from("--home"),other.into_os_string(),
        ];
        assert_eq!(home(args),Some(temp));
        assert_eq!(home([
            OsString::from("--home"),std::env::temp_dir().into_os_string(),
            OsString::from("--game-dir"),
            std::env::temp_dir().join("unavailable-vsclient-instance").into_os_string()
        ]),None);
    }
}
