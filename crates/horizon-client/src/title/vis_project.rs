//! Data-only project menus; no scripts or executable actions.
use serde::Deserialize;
use std::sync::OnceLock;
#[derive(Deserialize)]
pub(super) struct Menu {
    pub server: String,
    #[serde(default)]
    pub buttons: Vec<Button>,
}
#[derive(Deserialize)]
pub(super) struct Button {
    pub action: String,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
}
impl Menu {
    pub fn parse(text: &str) -> Option<Self> {
        if text.len() > 16384 {
            return None;
        }
        let menu: Self = serde_json::from_str(text).ok()?;
        if menu.server.is_empty()
            || menu.server.len() > 255
            || !menu
                .server
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-_:[]".contains(&b))
            || menu.buttons.is_empty()
            || menu.buttons.len() > 12
        {
            return None;
        }
        let mut play = false;
        for b in &menu.buttons {
            if !["play", "settings", "quit", "friends"].contains(&b.action.as_str())
                || b.label.chars().count() > 40
                || b.label.chars().any(char::is_control)
                || ![b.x, b.y, b.width].iter().all(|x| x.is_finite())
                || !(0.0..=1.0).contains(&b.x)
                || !(0.0..=1.0).contains(&b.y)
                || !(0.1..=1.0).contains(&b.width)
            {
                return None;
            }
            play |= b.action == "play";
        }
        play.then_some(menu)
    }
}
pub(super) fn config() -> Option<&'static Menu> {
    static MENU: OnceLock<Option<Menu>> = OnceLock::new();
    MENU.get_or_init(|| {
        let path = std::path::PathBuf::from(std::env::var_os("VIS_PROJECT_MENU")?);
        let metadata = std::fs::symlink_metadata(&path).ok()?;
        if !path.is_absolute()
            || !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > 16384
        {
            return None;
        }
        Menu::parse(&std::fs::read_to_string(path).ok()?)
    })
    .as_ref()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_menus_reject_executable_actions_and_invalid_geometry() {
        let valid = r#"{"server":"play.example:25565","buttons":[{"action":"play","label":"Jugar","x":0.5,"y":0.5,"width":0.6}]}"#;
        assert!(Menu::parse(valid).is_some());
        assert!(Menu::parse(&valid.replace("play.example:25565", "host;exec")).is_none());
        assert!(Menu::parse(&valid.replace("\"play\"", "\"exec\"")).is_none());
        assert!(Menu::parse(&valid.replace("0.6", "5.0")).is_none());
    }
}
