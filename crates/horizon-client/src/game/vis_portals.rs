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
    origin: Option<(f32, f32)>,
    drag: Option<(f32, f32)>,
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
        if self.snapshot["auto_pause"] != value["auto_pause"] {
            self.run_world = value["auto_pause"] == false;
        }
        let id = value["notice"]["id"].as_str().unwrap_or("");
        if self.snapshot.is_object() && id != self.last_notice && value["quiet"] != true {
            self.notice = Some(std::time::Instant::now());
        }
        self.last_notice = id.to_owned();
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
        if self.portals.is_none() {
            return false;
        }
        if self.screen == Screen::Friends {
            let (w, h) = self.gui_size();
            let width = (w - 12.0).max(1.0).min(500.0);
            let height = (h - 16.0).max(1.0);
            let (x, y) = self.vis_ui.origin.unwrap_or(((w - width) / 2.0, 8.0));
            match event {
                WindowEvent::MouseWheel { delta, .. } => {
                    let amount = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, dy) => *dy,
                        winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 20.0,
                    };
                    let count = self.vis_ui.snapshot["friends"]
                        .as_array()
                        .map_or(0, Vec::len);
                    if self.cursor.0 > x + width * 0.35 {
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
                WindowEvent::MouseInput {
                    state,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    if *state == ElementState::Pressed
                        && ui::hovered(self.cursor, x, y, width, 20.0)
                    {
                        self.vis_ui.drag = Some((self.cursor.0 - x, self.cursor.1 - y));
                        return true;
                    }
                    if *state == ElementState::Released && self.vis_ui.drag.take().is_some() {
                        return true;
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some((dx, dy)) = self.vis_ui.drag {
                        let scale = self.gui_scale().map_or(1.0, |g| g.scale);
                        let px = position.x as f32 / scale;
                        let py = position.y as f32 / scale;
                        self.vis_ui.origin = Some((
                            (px - dx).clamp(0.0, (w - width).max(0.0)),
                            (py - dy).clamp(0.0, (h - height).max(0.0)),
                        ));
                        self.cursor = (px, py);
                        return true;
                    }
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
                            if !self.vis_ui.selected.is_empty()
                                && !self.vis_ui.draft.trim().is_empty()
                            {
                                let event = json!({"type":"send","to":self.vis_ui.selected,"body":self.vis_ui.draft});
                                self.vis_ui.emit(event);
                                self.vis_ui.draft.clear();
                            }
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
        let atlas = &self.gui.atlas;
        let w = (batch.width - 12.0).clamp(200.0, 500.0);
        let h = (batch.height - 16.0).max(100.0);
        let (x, y) = self.vis_ui.origin.unwrap_or(((batch.width - w) / 2.0, 8.0));
        widgets::begin_screen(|| widgets::ScreenInfo::new("vis-portals", "VIS Portals"));
        batch.fill(atlas, x, y, w, h, rgba(18, 13, 32, 242));
        batch.text(atlas, "VIS PORTALS", x + 8.0, y + 7.0, u32::MAX, true);
        let controls = [
            ("close", tr("vis.portals.close", "Close")),
            ("pause", tr("vis.portals.pause", "Pause / Resume")),
            (
                "mute",
                if self.vis_ui.snapshot["voice"]["muted"] == true {
                    tr("vis.voice.muted", "Mic: OFF")
                } else {
                    tr("vis.voice.unmuted", "Mic: ON")
                },
            ),
            (
                "deafen",
                if self.vis_ui.snapshot["voice"]["deaf"] == true {
                    tr("vis.voice.deaf", "Audio: OFF")
                } else {
                    tr("vis.voice.listening", "Audio: ON")
                },
            ),
        ];
        for (i, (_id, label)) in controls.iter().enumerate() {
            let bx = x + 8.0 + i as f32 * (w - 16.0) / 4.0;
            let bw = (w - 20.0) / 4.0;

            ui::button(
                batch,
                atlas,
                bx,
                y + 22.0,
                bw,
                18.0,
                &Text::literal(label),
                true,
                ui::hovered(self.cursor, bx, y + 22.0, bw, 18.0),
            );
        }
        let left = (w * 0.35).min(150.0);
        let rows = ((h - 110.0) / 20.0).max(0.0) as usize;
        if let Some(friends) = self.vis_ui.snapshot["friends"].as_array() {
            for (i, f) in friends
                .iter()
                .skip(self.vis_ui.friend_offset)
                .take(rows)
                .enumerate()
            {
                let user = f["username"].as_str().unwrap_or("");
                let label = format!(
                    "{} · {}",
                    f["name"].as_str().unwrap_or(user),
                    f["presence"].as_str().unwrap_or("offline")
                );
                let by = y + 48.0 + i as f32 * 20.0;

                ui::button(
                    batch,
                    atlas,
                    x + 8.0,
                    by,
                    left - 10.0,
                    18.0,
                    &Text::literal(&label),
                    true,
                    ui::hovered(self.cursor, x + 8.0, by, left - 10.0, 18.0),
                );
            }
        }
        let tx = x + left + 6.0;
        batch.text(atlas, &self.vis_ui.selected, tx, y + 48.0, u32::MAX, true);
        if let Some(error) = self.vis_ui.snapshot["error"]
            .as_str()
            .filter(|v| !v.is_empty())
        {
            let text: String = error.chars().take(60).collect();
            batch.text(
                atlas,
                &text,
                x + 8.0,
                y + h - 27.0,
                rgba(255, 140, 140, 255),
                true,
            );
        }
        if let Some(invites) = self.vis_ui.snapshot["invitations"].as_array() {
            for (i, invite) in invites.iter().take(2).enumerate() {
                let by = y + 62.0 + i as f32 * 20.0;
                let bw = (w - left - 16.0) / 2.0;
                let join = format!(
                    "{} · {}",
                    invite["sender"].as_str().unwrap_or(""),
                    tr("vis.portals.join", "Join")
                );
                let decline = tr("vis.portals.decline", "Decline");
                for (j, label) in [join, decline].iter().enumerate() {
                    let bx = tx + j as f32 * (bw + 2.0);

                    ui::button(
                        batch,
                        atlas,
                        bx,
                        by,
                        bw,
                        18.0,
                        &Text::literal(label),
                        true,
                        ui::hovered(self.cursor, bx, by, bw, 18.0),
                    );
                }
            }
        }
        if let Some(messages) = self.vis_ui.snapshot["messages"].as_array() {
            let count = ((h - 180.0) / 10.0).max(0.0) as usize;
            let mut lines = Vec::new();
            for message in messages
                .iter()
                .filter(|m| m["from"] == self.vis_ui.selected || m["to"] == self.vis_ui.selected)
            {
                let text = format!(
                    "{}: {}",
                    message["from"].as_str().unwrap_or(""),
                    message["body"].as_str().unwrap_or("")
                );
                lines.extend(horizon_client::multiplayer::wrap(
                    &text,
                    (w - left - 15.0).max(20.0),
                    atlas,
                ));
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
                    y + 105.0 + i as f32 * 10.0,
                    rgba(220, 215, 235, 255),
                    true,
                );
            }
        }
        let input: String = if self.vis_ui.searching {
            &self.vis_ui.search
        } else {
            &self.vis_ui.draft
        }
        .chars()
        .rev()
        .take(55)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
        batch.text(atlas, &input, tx, y + h - 42.0, u32::MAX, true);
        let actions = [
            (
                "voice",
                if self.vis_ui.snapshot["voice"]["connected"] == true {
                    tr("vis.voice.disconnect", "Leave voice")
                } else {
                    tr("vis.voice.connect", "Join voice")
                },
            ),
            ("invite", tr("vis.portals.invite", "Invite")),
            ("presence", tr("vis.portals.status", "Status")),
            ("mode", tr("vis.voice.mode", "PTT / Open mic")),
            ("devices", tr("vis.voice.devices", "Devices")),
        ];
        for (i, (_id, label)) in actions.iter().enumerate() {
            let bx = x + 8.0 + i as f32 * (w - 16.0) / 5.0;
            let by = y + h - 66.0;
            let bw = (w - 20.0) / 5.0;

            ui::button(
                batch,
                atlas,
                bx,
                by,
                bw,
                18.0,
                &Text::literal(label),
                true,
                ui::hovered(self.cursor, bx, by, bw, 18.0),
            );
        }
        let footer = format!(
            "{} · {}",
            tr("vis.portals.hint", "Enter: send · Ctrl+F: search"),
            self.vis_ui.snapshot["hotkey"].as_str().unwrap_or("F8")
        );
        batch.text(
            atlas,
            &footer,
            x + 8.0,
            y + h - 15.0,
            rgba(170, 160, 190, 255),
            true,
        );
    }
    pub(super) fn click_vis_portals(&mut self) {
        let (w, h) = self.gui_size();
        let width = (w - 12.0).clamp(200.0, 500.0);
        let (x, y) = self.vis_ui.origin.unwrap_or(((w - width) / 2.0, 8.0));
        for i in 0..4 {
            let bx = x + 8.0 + i as f32 * (width - 16.0) / 4.0;
            if ui::hovered(self.cursor, bx, 30.0, (width - 20.0) / 4.0, 18.0) {
                match i {
                    0 => {
                        self.screen = Screen::Playing;
                        self.capture(self.title.is_none());
                    }
                    1 => {
                        if self.has_singleplayer_server() && !self.is_published() {
                            self.vis_ui.run_world = !self.vis_ui.run_world;
                        }
                    }
                    2 => self.vis_ui.emit(json!({"type":"voice","action":"mute"})),
                    _ => self.vis_ui.emit(json!({"type":"voice","action":"deafen"})),
                }
                return;
            }
        }
        for i in 0..5 {
            if ui::hovered(
                self.cursor,
                x + 8.0 + i as f32 * (width - 16.0) / 5.0,
                y + (h - 16.0).max(100.0) - 66.0,
                (width - 20.0) / 5.0,
                18.0,
            ) {
                match i {
                    0 => {
                        let action = if self.vis_ui.snapshot["voice"]["connected"] == true {
                            "leave"
                        } else {
                            "join"
                        };
                        self.vis_ui.emit(json!({"type":"voice","action":action}));
                    }
                    1 => {
                        if !self.vis_ui.selected.is_empty() {
                            self.vis_ui
                                .emit(json!({"type":"invite","to":self.vis_ui.selected}));
                        }
                    }
                    2 => {
                        let next = match self.vis_ui.snapshot["presence"].as_str() {
                            Some("online") => "away",
                            Some("away") => "busy",
                            Some("busy") => "invisible",
                            _ => "online",
                        };
                        self.vis_ui.emit(json!({"type":"presence","value":next,"custom_status":self.vis_ui.draft}));
                    }
                    3 => {
                        let next = if self.vis_ui.snapshot["voice"]["mode"] == "open" {
                            "ptt"
                        } else {
                            "open"
                        };
                        self.vis_ui
                            .emit(json!({"type":"voice","action":"mode","value":next}));
                    }
                    _ => self
                        .vis_ui
                        .emit(json!({"type":"voice","action":"settings"})),
                }
                return;
            }
        }
        let left = (width * 0.35).min(150.0);
        if let Some(invites) = self.vis_ui.snapshot["invitations"].as_array() {
            let action = invites
                .iter()
                .take(2)
                .enumerate()
                .find(|(i, _)| {
                    ui::hovered(
                        self.cursor,
                        x + left + 6.0,
                        y + 62.0 + *i as f32 * 20.0,
                        width - left - 12.0,
                        18.0,
                    )
                })
                .map(|(_, v)| v["id"].clone());
            if let Some(id) = action {
                let action = if self.cursor.0 >= x + left + 6.0 + (width - left - 16.0) / 2.0 + 2.0
                {
                    "decline"
                } else {
                    "join"
                };
                self.vis_ui.emit(json!({"type":action,"id":id}));
                return;
            }
        }

        let rows = (((h - 16.0).max(100.0) - 110.0) / 20.0).max(0.0) as usize;
        if let Some(friends) = self.vis_ui.snapshot["friends"].as_array() {
            for (i, f) in friends
                .iter()
                .skip(self.vis_ui.friend_offset)
                .take(rows)
                .enumerate()
            {
                if ui::hovered(
                    self.cursor,
                    x + 8.0,
                    y + 48.0 + i as f32 * 20.0,
                    left - 10.0,
                    18.0,
                ) {
                    self.vis_ui.chat_offset = 0;
                    self.vis_ui.selected = f["username"].as_str().unwrap_or("").to_owned();
                    return;
                }
            }
        }
    }
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
