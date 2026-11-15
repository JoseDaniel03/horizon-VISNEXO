//! Native VIS Portals panel. Social credentials stay in VIS Nexo.
use super::{Game, Screen};
use horizon_client::{
    hud::{HudBatch, rgba},
    lang,
    text::Text,
    ui, widgets,
};
use serde_json::{Value, json};
use std::collections::VecDeque;
mod layout;
use layout::Panel;
use winit::{
    event::{ElementState, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
#[derive(Default)]
pub(super) struct State {
    pub snapshot: Value,
    pub events: VecDeque<Value>,
    pub selected: String,
    friend_offset: usize,
    chat_offset: usize,
    pub draft: String,
    search: String,
    searching: bool,
    pub run_world: bool,
    notice: Option<std::time::Instant>,
    last_notice: String,
    pub received: Option<std::time::Instant>,
}
impl State {
    fn enter_text(&mut self, text: &str) {
        let target = if self.searching {
            &mut self.search
        } else {
            &mut self.draft
        };
        let max = if self.searching { 64 } else { 1300 };
        for ch in text.chars().filter(|c| !c.is_control()) {
            if target.len() + ch.len_utf8() <= max {
                target.push(ch);
            }
        }
        if self.searching {
            self.emit(json!({"type":"search","query":self.search}));
            self.friend_offset = 0;
        }
    }
    fn emit(&mut self, event: Value) {
        if self.events.len() < 32 {
            self.events.push_back(event);
        }
    }
    pub fn update(&mut self, value: Value) -> Result<(), &'static str> {
        if !value.is_object() || value.to_string().len() > 12000 {
            return Err("invalid social snapshot");
        }
        let disconnected = value["connected"] == false;
        let account_changed = self.snapshot["account"]["username"] != value["account"]["username"];
        if disconnected || account_changed {
            self.selected.clear();
            self.draft.clear();
            self.search.clear();
            self.chat_offset = 0;
            self.friend_offset = 0;
            self.events.clear();
            self.notice = None;
            self.last_notice.clear();
        }
        if self.snapshot["auto_pause"] != value["auto_pause"] {
            self.run_world = value["auto_pause"] == false;
        }
        let id = value["notice"]["id"].as_str().unwrap_or("");
        if self.snapshot.is_object() && id != self.last_notice && value["quiet"] != true {
            self.notice = Some(std::time::Instant::now());
        }
        self.last_notice = id.to_owned();
        if disconnected {
            self.notice = None;
        }
        self.friend_offset = self.friend_offset.min(
            value["friends"]
                .as_array()
                .map_or(0, |a| a.len().saturating_sub(1)),
        );
        self.received = Some(std::time::Instant::now());
        self.snapshot = value;
        Ok(())
    }
}
fn tr(key: &str, fallback: &str) -> String {
    lang::get_or(key, fallback)
}
impl Game<'_> {
    pub(super) fn vis_input(&mut self, event: &WindowEvent) -> bool {
        if self.vis_portals_bridge.is_none() {
            return false;
        }
        if self.screen == Screen::Friends {
            let (w, h) = self.gui_size();
            let panel = Panel::new(w, h);
            let x = panel.x;
            match event {
                WindowEvent::MouseWheel { delta, .. } => {
                    let amount = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, dy) => *dy,
                        winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 20.0,
                    };
                    let count = self.vis_ui.snapshot["friends"]
                        .as_array()
                        .map_or(0, Vec::len);
                    if self.cursor.0 > x + panel.left {
                        if amount > 0.0 {
                            self.vis_ui.chat_offset = self.vis_ui.chat_offset.saturating_add(3);
                        } else {
                            self.vis_ui.chat_offset = self.vis_ui.chat_offset.saturating_sub(3);
                        }
                        return true;
                    }
                    if amount > 0.0 {
                        self.vis_ui.friend_offset = self.vis_ui.friend_offset.saturating_sub(3);
                    } else if amount < 0.0 {
                        self.vis_ui.friend_offset =
                            (self.vis_ui.friend_offset + 3).min(count.saturating_sub(1));
                    }
                    return true;
                }
                WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                    self.vis_ui.enter_text(text);
                    return true;
                }
                _ => {}
            }
        }
        if let WindowEvent::Focused(false) = event {
            self.vis_ui
                .emit(json!({"type":"voice","action":"ptt","pressed":false}));
        }
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if event.physical_key == PhysicalKey::Code(KeyCode::KeyV)
                && self.screen == Screen::Playing
                && !self.screen_over_playing()
                && self.vis_ui.snapshot["voice"]["connected"] == true
            {
                self.vis_ui.emit(json!({"type":"voice","action":"ptt","pressed":event.state==ElementState::Pressed}));
                return true;
            }
            if event.physical_key
                == PhysicalKey::Code(match self.vis_ui.snapshot["hotkey"].as_str() {
                    Some("F6") => KeyCode::F6,
                    Some("F7") => KeyCode::F7,
                    Some("F9") => KeyCode::F9,
                    Some("F10") => KeyCode::F10,
                    _ => KeyCode::F8,
                })
                && event.state == ElementState::Pressed
                && !event.repeat
            {
                if self.screen == Screen::Friends {
                    self.screen = Screen::Playing;
                    self.capture(self.title.is_none());
                } else if self.screen == Screen::Playing || self.screen == Screen::Paused {
                    self.vis_ui
                        .emit(json!({"type":"voice","action":"ptt","pressed":false}));
                    self.keys.clear();
                    self.physical_keys.clear();
                    self.physical_buttons.clear();
                    self.left = false;
                    self.right = false;
                    self.screen = Screen::Friends;
                    self.capture(false);
                }
                return true;
            }
            if self.screen == Screen::Friends {
                if event.state == ElementState::Pressed {
                    if event.physical_key == PhysicalKey::Code(KeyCode::KeyF)
                        && self.modifiers.control_key()
                    {
                        self.vis_ui.searching = !self.vis_ui.searching;
                        return true;
                    }

                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => {
                            if self.vis_ui.searching {
                                self.vis_ui.searching = false;
                                return true;
                            }
                            self.screen = Screen::Playing;
                            self.capture(self.title.is_none());
                        }
                        PhysicalKey::Code(KeyCode::Backspace) => {
                            if self.vis_ui.searching {
                                self.vis_ui.search.pop();
                                self.vis_ui
                                    .emit(json!({"type":"search","query":self.vis_ui.search}));
                            } else {
                                self.vis_ui.draft.pop();
                            }
                        }
                        PhysicalKey::Code(KeyCode::Enter) => {
                            if self.vis_ui.searching {
                                self.vis_ui.searching = false;
                                return true;
                            }
                            self.vis_send();
                        }
                        _ => {
                            if let Some(text) = event.text.as_deref() {
                                self.vis_ui.enter_text(text);
                            }
                        }
                    }
                }
                return true;
            }
        }
        false
    }
    pub(super) fn draw_vis_portals(&mut self, batch: &mut HudBatch) {
        let p = Panel::new(batch.width, batch.height);
        let atlas = &self.gui.atlas;
        let connected = self.vis_ui.snapshot["connected"] == true;
        let title = tr("vis.portals.title", "Play with friends");
        widgets::begin_screen(|| widgets::ScreenInfo::new("vis-portals", title.clone()));
        ui::menu_background(
            batch,
            atlas,
            0.,
            0.,
            batch.width,
            batch.height,
            self.title.is_none(),
        );
        ui::separators(
            batch,
            atlas,
            p.y + 58.,
            p.y + p.h - 119.,
            self.title.is_none(),
        );
        batch.text(
            atlas,
            &title,
            (batch.width - HudBatch::text_width(atlas, &title)) / 2.,
            p.y + 9.,
            u32::MAX,
            true,
        );
        let identity = self.vis_ui.snapshot["account"]["display_name"]
            .as_str()
            .unwrap_or("");
        let status = if connected {
            tr("vis.portals.connected", "Connected to Nexo")
        } else {
            tr("vis.portals.disconnected", "Nexo disconnected")
        };
        let subtitle = fit(atlas, &format!("{identity} · {status}"), p.w - 16.);
        batch.text(
            atlas,
            &subtitle,
            p.x + 8.,
            p.y + 23.,
            if connected {
                rgba(69, 194, 122, 255)
            } else {
                rgba(240, 122, 104, 255)
            },
            false,
        );
        for (action, r, label, enabled) in self.vis_buttons(p) {
            let [x, y, w, h] = r;
            let _ = action;
            ui::button(
                batch,
                atlas,
                x,
                y,
                w,
                h,
                &Text::literal(fit(atlas, &label, w - 8.)),
                enabled,
                ui::hovered(self.cursor, x, y, w, h),
            );
        }
        if p.compact() {
            batch.text(
                atlas,
                &fit(
                    atlas,
                    &tr("vis.portals.expand", "Enlarge the window to show friends"),
                    p.w - 16.,
                ),
                p.x + 8.,
                p.y + 64.,
                rgba(159, 163, 174, 255),
                false,
            );
            return;
        }
        batch.fill(
            atlas,
            p.x + p.left,
            p.y + 62.,
            1.,
            p.h - 158.,
            rgba(128, 128, 128, 255),
        );
        let count = self.vis_ui.snapshot["friends"]
            .as_array()
            .map_or(0, Vec::len);
        batch.text(
            atlas,
            &format!("{} · {count}", tr("vis.portals.contacts", "Friends")),
            p.x + 8.,
            p.y + 61.,
            rgba(159, 163, 174, 255),
            false,
        );
        if let Some(friends) = self.vis_ui.snapshot["friends"].as_array() {
            for (i, f) in friends
                .iter()
                .skip(self.vis_ui.friend_offset)
                .take(p.rows())
                .enumerate()
            {
                let [x, y, w, h] = p.row(i);
                let user = f["username"].as_str().unwrap_or("");
                let selected = user == self.vis_ui.selected;
                if selected {
                    batch.fill(atlas, x, y, w, h, rgba(100, 100, 100, 140));
                }
                ui::button(
                    batch,
                    atlas,
                    x,
                    y,
                    w,
                    h,
                    &Text::literal(fit(atlas, f["name"].as_str().unwrap_or(user), w - 12.)),
                    connected,
                    ui::hovered(self.cursor, x, y, w, h),
                );
                let color = match f["presence"].as_str() {
                    Some("online") => rgba(69, 194, 122, 255),
                    Some("away") => rgba(224, 181, 84, 255),
                    Some("busy") => rgba(240, 122, 104, 255),
                    _ => rgba(139, 143, 156, 255),
                };
                batch.fill(atlas, x + 2., y + 8., 3., 6., color);
            }
        }
        if count == 0 {
            batch.text(
                atlas,
                &fit(
                    atlas,
                    &if self.vis_ui.received.is_none() {
                        tr("vis.portals.waiting", "Waiting for Nexo…")
                    } else if !connected {
                        tr("vis.portals.offline_hint", "Connect your account in Nexo")
                    } else {
                        tr("vis.portals.empty", "No friends to show")
                    },
                    p.left - 16.,
                ),
                p.x + 8.,
                p.y + 78.,
                rgba(159, 163, 174, 255),
                false,
            );
        }
        let tx = p.chat_x();
        let chat_width = p.w - p.left - 20.;
        let title = if self.vis_ui.selected.is_empty() {
            tr("vis.portals.select", "Select a friend")
        } else {
            self.vis_ui.selected.clone()
        };
        batch.text(
            atlas,
            &fit(atlas, &title, chat_width),
            tx,
            p.y + 61.,
            u32::MAX,
            false,
        );
        if let Some(invites) = self.vis_ui.snapshot["invitations"].as_array() {
            for (i, invite) in invites.iter().take(2).enumerate() {
                for (j, key) in ["join", "decline"].iter().enumerate() {
                    let bx = tx + j as f32 * (chat_width / 2.);
                    let by = p.y + 75. + i as f32 * 20.;
                    let label = if j == 0 {
                        format!(
                            "{} · {}",
                            invite["sender"].as_str().unwrap_or(""),
                            tr("vis.portals.join", "Join")
                        )
                    } else {
                        tr("vis.portals.decline", "Decline")
                    };
                    ui::button(
                        batch,
                        atlas,
                        bx,
                        by,
                        chat_width / 2. - 2.,
                        18.,
                        &Text::literal(fit(atlas, &label, chat_width / 2. - 8.)),
                        connected && (j == 1 || self.title.is_some()),
                        ui::hovered(self.cursor, bx, by, chat_width / 2. - 2., 18.),
                    );
                    let _ = key;
                }
            }
        }
        let invite_count = self.vis_ui.snapshot["invitations"]
            .as_array()
            .map_or(0, |a| a.len().min(2));
        let chat_y = p.y + 78. + invite_count as f32 * 20.;
        let count = ((p.content_bottom() - chat_y) / 11.).max(0.) as usize;
        let mut lines = Vec::new();
        if let Some(messages) = self.vis_ui.snapshot["messages"].as_array() {
            for message in messages.iter().filter(|m| {
                !self.vis_ui.selected.is_empty()
                    && (m["from"] == self.vis_ui.selected || m["to"] == self.vis_ui.selected)
            }) {
                lines.extend(horizon_client::multiplayer::wrap(
                    &format!(
                        "{}: {}",
                        message["from"].as_str().unwrap_or(""),
                        message["body"].as_str().unwrap_or("")
                    ),
                    chat_width,
                    atlas,
                ));
            }
        }
        self.vis_ui.chat_offset = self
            .vis_ui
            .chat_offset
            .min(lines.len().saturating_sub(count));
        let start = lines.len().saturating_sub(count + self.vis_ui.chat_offset);
        for (i, line) in lines.iter().skip(start).take(count).enumerate() {
            batch.text(
                atlas,
                line,
                tx,
                chat_y + i as f32 * 11.,
                rgba(197, 199, 209, 255),
                false,
            );
        }
        batch.fill(
            atlas,
            tx,
            p.y + p.h - 113.,
            chat_width,
            20.,
            rgba(0, 0, 0, 180),
        );
        let input = if self.vis_ui.searching {
            &self.vis_ui.search
        } else {
            &self.vis_ui.draft
        };
        let input = if input.is_empty() {
            if self.vis_ui.searching {
                tr("vis.portals.search_hint", "Search friends…")
            } else {
                tr("vis.portals.message_hint", "Write a message…")
            }
        } else {
            input.clone()
        };
        batch.text(
            atlas,
            &fit(atlas, &input, chat_width - 8.),
            tx + 4.,
            p.y + p.h - 107.,
            rgba(197, 199, 209, 255),
            false,
        );
        let error = self.vis_ui.snapshot["error"].as_str().unwrap_or("");
        let footer = if error.is_empty() {
            tr("vis.portals.hint", "Enter: send · Ctrl+F: search")
        } else {
            error.to_owned()
        };
        batch.text(
            atlas,
            &fit(atlas, &footer, p.w - 16.),
            p.x + 8.,
            p.y + p.h - 15.,
            rgba(159, 163, 174, 255),
            false,
        );
    }
    fn vis_buttons(&self, p: Panel) -> Vec<(usize, [f32; 4], String, bool)> {
        let online = self.vis_ui.snapshot["connected"] == true;
        let private = self.has_singleplayer_server() && !self.is_published();
        let mut buttons = vec![(0, p.top(0), tr("vis.portals.close", "Close"), true)];
        if p.compact() {
            return buttons;
        }
        buttons.extend([
            (
                1,
                p.top(1),
                if self.vis_ui.run_world {
                    tr("vis.portals.pause", "Pause world")
                } else {
                    tr("vis.portals.resume", "Resume world")
                },
                private,
            ),
            (
                2,
                p.top(2),
                if self.vis_ui.snapshot["voice"]["muted"] == true {
                    tr("vis.voice.muted", "Mic: OFF")
                } else {
                    tr("vis.voice.unmuted", "Mic: ON")
                },
                online,
            ),
            (
                3,
                p.top(3),
                if self.vis_ui.snapshot["voice"]["deaf"] == true {
                    tr("vis.voice.deaf", "Audio: OFF")
                } else {
                    tr("vis.voice.listening", "Audio: ON")
                },
                online,
            ),
            (
                4,
                p.action(0),
                if self.vis_ui.snapshot["voice"]["connected"] == true {
                    tr("vis.voice.disconnect", "Leave voice")
                } else {
                    tr("vis.voice.connect", "Join voice")
                },
                online,
            ),
            (
                5,
                p.action(1),
                tr("vis.portals.invite", "Invite"),
                online
                    && !self.vis_ui.selected.is_empty()
                    && self.title.is_none()
                    && self.has_singleplayer_server(),
            ),
            (
                6,
                p.action(2),
                if online {
                    tr("vis.portals.search", "Search friends")
                } else {
                    tr("vis.portals.refresh", "Reconnect")
                },
                true,
            ),
            (
                7,
                p.action(3),
                tr("vis.voice.mode", "PTT / Open mic"),
                online,
            ),
            (8, p.action(4), tr("vis.voice.devices", "Devices"), online),
            (
                9,
                p.action(5),
                if self.vis_ui.snapshot["world_sharing"] == true {
                    tr("vis.portals.stop", "Stop sharing")
                } else {
                    tr("vis.portals.send", "Send")
                },
                online
                    && (self.vis_ui.snapshot["world_sharing"] == true
                        || (!self.vis_ui.selected.is_empty()
                            && !self.vis_ui.draft.trim().is_empty())),
            ),
        ]);
        buttons
    }
    pub(super) fn click_vis_portals(&mut self) {
        let (w, h) = self.gui_size();
        let p = Panel::new(w, h);
        for (id, [x, y, w, h], _, enabled) in self.vis_buttons(p) {
            if enabled && ui::hovered(self.cursor, x, y, w, h) {
                match id {
                    0 => {
                        self.screen = Screen::Playing;
                        self.capture(self.title.is_none());
                    }
                    1 => self.vis_ui.run_world = !self.vis_ui.run_world,
                    2 => self.vis_ui.emit(json!({"type":"voice","action":"mute"})),
                    3 => self.vis_ui.emit(json!({"type":"voice","action":"deafen"})),
                    4 => {
                        let action = if self.vis_ui.snapshot["voice"]["connected"] == true {
                            "leave"
                        } else {
                            "join"
                        };
                        self.vis_ui.emit(json!({"type":"voice","action":action}));
                    }
                    5 => self
                        .vis_ui
                        .emit(json!({"type":"invite","to":self.vis_ui.selected})),
                    6 => {
                        self.vis_ui.emit(json!({"type":"refresh"}));
                        if self.vis_ui.snapshot["connected"] == true {
                            self.vis_ui.searching = !self.vis_ui.searching;
                        }
                    }
                    7 => {
                        let next = if self.vis_ui.snapshot["voice"]["mode"] == "open" {
                            "ptt"
                        } else {
                            "open"
                        };
                        self.vis_ui
                            .emit(json!({"type":"voice","action":"mode","value":next}));
                    }
                    8 => self
                        .vis_ui
                        .emit(json!({"type":"voice","action":"settings"})),
                    _ => {
                        if self.vis_ui.snapshot["world_sharing"] == true {
                            self.vis_ui.emit(json!({"type":"stop"}));
                        } else {
                            self.vis_send();
                        }
                    }
                }
                return;
            }
        }
        if p.compact() || self.vis_ui.snapshot["connected"] != true {
            return;
        }
        if let Some(invites) = self.vis_ui.snapshot["invitations"].as_array() {
            let tx = p.chat_x();
            let width = p.w - p.left - 20.;
            let picked = invites.iter().take(2).enumerate().find_map(|(i, invite)| {
                for j in 0..2 {
                    if (j == 1 || self.title.is_some())
                        && ui::hovered(
                            self.cursor,
                            tx + j as f32 * width / 2.,
                            p.y + 75. + i as f32 * 20.,
                            width / 2. - 2.,
                            18.,
                        )
                    {
                        return Some((invite["id"].clone(), j));
                    }
                }
                None
            });
            if let Some((id, j)) = picked {
                self.vis_ui
                    .emit(json!({"type":if j==0{"join"}else{"decline"},"id":id}));
                return;
            }
        }
        if let Some(friends) = self.vis_ui.snapshot["friends"].as_array() {
            let picked = friends
                .iter()
                .skip(self.vis_ui.friend_offset)
                .take(p.rows())
                .enumerate()
                .find_map(|(i, f)| {
                    let [x, y, w, h] = p.row(i);
                    ui::hovered(self.cursor, x, y, w, h)
                        .then(|| f["username"].as_str().unwrap_or("").to_owned())
                });
            if let Some(user) = picked {
                self.vis_ui.selected = user;
                self.vis_ui.chat_offset = 0;
                self.vis_ui.draft.clear();
                self.vis_ui
                    .emit(json!({"type":"select","username":self.vis_ui.selected}));
            }
        }
    }
    fn vis_send(&mut self) {
        if self.vis_ui.snapshot["connected"] == true
            && !self.vis_ui.selected.is_empty()
            && !self.vis_ui.draft.trim().is_empty()
        {
            self.vis_ui
                .emit(json!({"type":"send","to":self.vis_ui.selected,"body":self.vis_ui.draft}));
            self.vis_ui.draft.clear();
        }
    }
}
fn fit(atlas: &horizon_client::hud::HudAtlas, text: &str, width: f32) -> String {
    let mut result = text.chars().take(160).collect::<String>();
    while !result.is_empty() && HudBatch::text_width(atlas, &result) > width.max(0.) {
        result.pop();
    }
    result
}

impl Game<'_> {
    pub(super) fn draw_vis_notice(&mut self, batch: &mut HudBatch) {
        if self.screen != Screen::Playing
            || !self
                .vis_ui
                .notice
                .is_some_and(|at| at.elapsed().as_secs() < 5)
        {
            return;
        }
        let text = format!(
            "VIS Portals · {}",
            self.vis_ui.snapshot["notice"]["from"]
                .as_str()
                .unwrap_or("")
        );
        batch.text(
            &self.gui.atlas,
            &text,
            8.0,
            8.0,
            rgba(220, 200, 255, 255),
            true,
        );
    }
}
