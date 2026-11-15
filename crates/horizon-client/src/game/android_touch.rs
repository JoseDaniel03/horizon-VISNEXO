//! Android translates touch into existing controller/mouse paths; simulation stays shared.
use super::*;
use horizon_client_render::touch::{
    TouchControls,
    layout::{Control, Settings},
};
use winit::event::{Touch, TouchPhase};
pub(super) fn controls() -> TouchControls {
    let mut controls = TouchControls::default();
    controls.settings = Settings::load(&Game::options_path().with_file_name("vis-touch.json"));
    controls
}

pub(super) struct Editor {
    pub(super) original: Settings,
    drag: Option<(u64, Control)>,
    selected: Control,
}

impl Game<'_> {
    pub(super) fn android_cancel_editor(&mut self) {
        if let Some(editor) = self.android_editor.take() {
            self.android_touch.settings = editor.original;
        }
        self.android_touch.clear();
        self.screen = Screen::Playing;
        self.capture(true);
    }
    fn android_touch_requests(&mut self) {
        for (requested, key) in [
            (
                std::mem::take(&mut self.android_touch.chat_requested),
                KeyCode::KeyT,
            ),
            (
                std::mem::take(&mut self.android_touch.debug_requested),
                KeyCode::F3,
            ),
            (
                std::mem::take(&mut self.android_touch.perspective_requested),
                KeyCode::F5,
            ),
        ] {
            if requested && let Err(error) = self.key_pressed(key, key) {
                log::warn!("Android shortcut: {error}");
            }
        }
        for (token, down) in self.android_touch.key_events() {
            if down && token == "action:key.swapOffhand" {
                if let Err(error) = self.key_pressed(KeyCode::KeyF, KeyCode::KeyF) {
                    log::warn!("Android offhand: {error}");
                }
            }
        }
        if std::mem::take(&mut self.android_touch.settings_requested) {
            self.android_editor = Some(Editor {
                original: self.android_touch.settings.clone(),
                drag: None,
                selected: Control::Move,
            });
            self.android_touch.clear();
            self.screen = Screen::Paused;
            self.capture(false);
            self.pause = self.pauses_game();
            if let Some(server) = &self.integrated {
                server.set_paused(self.pause);
            }
        }
        self.sync_ime();
    }
    fn android_edit_touch(&mut self, touch: &Touch) {
        let Some(window) = &self.window else { return };
        let size = window.inner_size();
        let point = [
            touch.location.x as f32 / size.width.max(1) as f32,
            touch.location.y as f32 / size.height.max(1) as f32,
        ];
        self.android_touch.viewport(size.width, size.height);
        if touch.phase == TouchPhase::Started && point[1] < 0.12 {
            let action = (point[0].clamp(0., 0.999) * 8.) as usize;
            match action {
                0 => {
                    if let Err(error) = self
                        .android_touch
                        .settings
                        .save(&Self::options_path().with_file_name("vis-touch.json"))
                    {
                        log::warn!("Android touch settings: {error}");
                        self.error = Some(error.into());
                        return;
                    }
                    self.android_editor = None;
                }
                1 => {
                    if let Some(editor) = self.android_editor.take() {
                        self.android_touch.settings = editor.original;
                    }
                }
                2 => self.android_touch.settings = Settings::default(),
                3 | 4 => {
                    let editor = self.android_editor.as_ref().unwrap();
                    let mode = self.android_touch.settings.mode.index();
                    let slot = self.android_touch.settings.slot(editor.selected);
                    let place = &mut self.android_touch.settings.layouts[mode][slot];
                    place.size =
                        (place.size + if action == 3 { -0.01 } else { 0.01 }).clamp(0.04, 0.5);
                }
                5 | 6 => {
                    let s = &mut self.android_touch.settings;
                    s.sensitivity =
                        (s.sensitivity + if action == 5 { -0.1 } else { 0.1 }).clamp(0.1, 4.0);
                }
                7 => {
                    self.android_touch.settings.left_handed =
                        !self.android_touch.settings.left_handed
                }
                _ => {}
            }
            if self.android_editor.is_none() {
                self.android_touch.clear();
                self.screen = Screen::Playing;
                self.capture(true);
            }
            return;
        }
        let Some(editor) = self.android_editor.as_mut() else {
            return;
        };
        if touch.phase == TouchPhase::Started {
            let aspect = size.width.max(1) as f32 / size.height.max(1) as f32;
            editor.drag = Control::ALL
                .into_iter()
                .rev()
                .find(|&c| {
                    self.android_touch.settings.visible(c)
                        && horizon_client_render::touch::layout::contains(
                            self.android_touch.placement(c).rect(aspect),
                            point,
                        )
                })
                .map(|c| {
                    editor.selected = c;
                    (touch.id, c)
                });
        }
        if let Some((finger, control)) = editor.drag
            && finger == touch.id
        {
            if touch.phase == TouchPhase::Moved {
                let mode = self.android_touch.settings.mode.index();
                // Placement coordinates are stored before the optional mirror.
                let x = if self.android_touch.settings.left_handed {
                    1.0 - point[0]
                } else {
                    point[0]
                };
                self.android_touch.settings.layouts[mode]
                    [self.android_touch.settings.slot(control)]
                .center = [x.clamp(0.04, 0.96), point[1].clamp(0.16, 0.94)];
            }
            if matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                editor.drag = None;
            }
        }
    }
}

impl Game<'_> {
    pub(super) fn android_touch_event(
        &mut self,
        events: &dyn EventLoopCtl,
        id: WindowId,
        touch: Touch,
        origin: input_trace::InputOrigin,
    ) {
        if self.android_editor.is_some() {
            self.android_edit_touch(&touch);
            return;
        }
        if self.title.is_none() && self.screen == Screen::Playing && !self.screen_over_playing() {
            if touch.phase == TouchPhase::Started {
                let (w, h) = self.gui_size();
                let scale = self.gui_scale().map_or(1.0, |g| g.scale);
                let point = [
                    touch.location.x as f32 / scale,
                    touch.location.y as f32 / scale,
                ];
                if let Some(slot) = horizon_client_render::touch::hotbar_slot(w, h, point) {
                    let keys = [
                        KeyCode::Digit1,
                        KeyCode::Digit2,
                        KeyCode::Digit3,
                        KeyCode::Digit4,
                        KeyCode::Digit5,
                        KeyCode::Digit6,
                        KeyCode::Digit7,
                        KeyCode::Digit8,
                        KeyCode::Digit9,
                    ];
                    if let Err(error) = self.key_pressed(keys[slot], keys[slot]) {
                        log::warn!("Android hotbar: {error}");
                    }
                    return;
                }
            }
            if let Some(window) = &self.window {
                let size = window.inner_size();
                self.android_touch.viewport(size.width, size.height);
                let delta = self.android_touch.update(
                    touch.id,
                    [
                        touch.location.x as f32 / size.width.max(1) as f32,
                        touch.location.y as f32 / size.height.max(1) as f32,
                    ],
                    touch.phase == TouchPhase::Started,
                    matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled),
                );
                let delta = horizon_client_render::touch::camera_delta(
                    delta,
                    size.width,
                    size.height,
                    &self.android_touch.settings,
                );
                self.look_turn(f64::from(delta[0]), f64::from(delta[1]));
                self.android_touch_requests();
            }
        } else {
            self.android_touch.clear();
            if touch.phase == TouchPhase::Started && self.android_menu_finger.is_none() {
                self.android_menu_finger = Some(touch.id);
            }
            if self.android_menu_finger != Some(touch.id) {
                return;
            }
            self.handle_window_event(
                events,
                id,
                WindowEvent::CursorMoved {
                    device_id: touch.device_id,
                    position: touch.location,
                },
                origin,
            );
            if touch.phase != TouchPhase::Moved {
                self.handle_window_event(
                    events,
                    id,
                    WindowEvent::MouseInput {
                        device_id: touch.device_id,
                        state: if touch.phase == TouchPhase::Started {
                            ElementState::Pressed
                        } else {
                            ElementState::Released
                        },
                        button: MouseButton::Left,
                    },
                    origin,
                );
            }
            if matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.android_menu_finger = None;
            }
        }
    }
    pub(super) fn android_touch_hud(&self, batch: &mut HudBatch) {
        let editing = self.android_editor.is_some();
        if self.title.is_some()
            || (!editing && (self.screen != Screen::Playing || self.screen_over_playing()))
        {
            return;
        }
        let (w, h) = self.gui_size();
        let atlas = &self.gui.atlas;
        if editing {
            batch.fill(atlas, 0., 0., w, h, 0xdd111111);
            for (i, key) in [
                "save", "cancel", "reset", "smaller", "larger", "slower", "faster", "mirror",
            ]
            .into_iter()
            .enumerate()
            {
                let x = i as f32 * w / 8.;
                batch.fill(atlas, x, 0., w / 8. - 1., h * 0.12, 0xff444444);
                batch.text(
                    atlas,
                    &horizon_client_render::lang::get(&format!("vis.touch.editor.{key}")),
                    x + 2.,
                    3.,
                    0xffffffff,
                    true,
                );
            }
        }
        for c in Control::ALL {
            if !self.android_touch.visible(c) {
                continue;
            }
            let [x, y, rw, rh] = self.android_touch.placement(c).rect(w / h.max(1.));
            let color = if self
                .android_editor
                .as_ref()
                .is_some_and(|editor| editor.selected == c)
            {
                0xcc776699
            } else {
                0x88333333
            };
            batch.fill(atlas, x * w, y * h, rw * w, rh * h, color);
            let label =
                horizon_client_render::lang::get(&format!("vis.touch.control.{}", c as usize));
            batch.text(atlas, &label, x * w + 2., y * h + 2., 0xffffffff, true);
        }
    }
}
