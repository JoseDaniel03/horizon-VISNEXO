//! Optional authoritative voice position publisher. Server-only credentials;
//! no network work runs on a simulation tick. Nothing starts without config.
use super::Game;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{
        OnceLock,
        mpsc::{SyncSender, sync_channel},
    },
    time::Duration,
};
struct Publisher {
    sender: SyncSender<Value>,
    identities: HashMap<u128, String>,
    project: String,
}
fn publisher() -> Option<&'static Publisher> {
    static PUBLISHER: OnceLock<Option<Publisher>> = OnceLock::new();
    PUBLISHER
        .get_or_init(|| {
            let base = std::env::var("VIS_VOICE_URL").ok()?;
            if !base.starts_with("https://") || base.contains(['\r', '\n', '@']) {
                return None;
            }
            let project = std::env::var("VIS_VOICE_PROJECT").ok()?;
            let token = std::env::var("VIS_VOICE_TOKEN").ok()?;
            if token.len() < 32 {
                return None;
            }
            let path = std::env::var_os("VIS_VOICE_IDENTITIES")?;
            let meta = std::fs::metadata(&path).ok()?;
            if meta.len() > 1024 * 1024 {
                return None;
            }
            let raw: HashMap<String, String> =
                serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
            let mut identities = HashMap::new();
            for (uuid, name) in raw {
                if !(3..=24).contains(&name.len())
                    || !name
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                {
                    return None;
                }
                let uuid = u128::from_str_radix(&uuid.replace('-', ""), 16).ok()?;
                identities.insert(uuid, name);
            }
            let url = format!("{}/v1/voice/positions", base.trim_end_matches('/'));
            let (sender, receiver) = sync_channel::<Value>(1);
            std::thread::Builder::new()
                .name("vis-voice-positions".into())
                .spawn(move || {
                    let http = ureq::AgentBuilder::new()
                        .redirects(0)
                        .timeout(Duration::from_secs(2))
                        .build();
                    while let Ok(value) = receiver.recv() {
                        // Fail closed: the service expires positions after three seconds.
                        let _ = http
                            .post(&url)
                            .set("authorization", &format!("Bearer {token}"))
                            .set("content-type", "application/json")
                            .send_string(&value.to_string());
                    }
                })
                .ok()?;
            Some(Publisher {
                sender,
                identities,
                project,
            })
        })
        .as_ref()
}
impl Game<'_> {
    pub(super) fn publish_vis_voice_positions(&self) {
        if self.remote.is_some() || self.ticks % 20 != 0 {
            return;
        }
        let Some(publisher) = publisher() else {
            return;
        };
        let Some(lan) = &self.lan else {
            return;
        };
        let players:Vec<_>=lan.hub.guests.iter().filter_map(|guest|{
   let username=publisher.identities.get(&guest.uuid)?;
   Some(json!({"username":username,"dimension":guest.dimension,"x":guest.position[0],"y":guest.position[1],"z":guest.position[2]}))
  }).take(100).collect();
        let _ = publisher
            .sender
            .try_send(json!({"project_id":publisher.project,"players":players}));
    }
}
