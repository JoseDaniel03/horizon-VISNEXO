/tmp/mcrust-mobile-ui/crates/horizon-client/src/game/android_touch.rs:

//! Android translates touch into existing controller/mouse paths; simulation stays shared.
use super::*;
use horizon_client_render::touch::{
    TouchControls,
    layout::{Control, HIDDEN},
};
use winit::event::{Touch, TouchPhase};
pub(super) fn controls() -> TouchControls {
    let mut controls = TouchControls::default();
    for c in [
        Control::Chat,
        Control::Settings,
        Control::Debug,
        Control::Perspective,
    ] {
        controls.settings.button_visibility[c as usize] = HIDDEN;
    }
    controls
}
// Both the startup menu and the in-world screens use the same hit rectangle.
pub(super) fn back_hit(
    batch: &HudBatch,
    point: winit::dpi::PhysicalPosition<f64>,
    size: [u32; 2],
) -> bool {
    let [x, y, side] = back_bounds(batch);
    let px = point.x as f32 * batch.width / size[0].max(1) as f32;
    let py = point.y as f32 * batch.height / size[1].max(1) as f32;
    px >= x && px < x + side && py >= y && py < y + side
}

fn back_bounds(batch: &HudBatch) -> [f32; 3] {
    let side = (batch.height * 0.11).max(22.0);
    [batch.width - side - 6.0, 6.0, side]
}

pub(super) fn draw_back(batch: &mut HudBatch, atlas: &horizon_client_render::hud::HudAtlas) {
    let [x, y, side] = back_bounds(batch);
    batch.fill(atlas, x, y, side, side, 0xe8242631);
    batch.sprite_scaled(
        atlas,
        "vis/touch/custom_cross",
        x,
        y,
        side,
        side,
        0xffffffff,
    );
}

fn icon(control: Control) -> Option<&'static str> {
    Some(match control {
        Control::Move => "vis/touch/stick",
        Control::Jump => "vis/touch/jump",
        Control::Sneak => "vis/touch/sneak",
        Control::Attack => "vis/touch/attack",
        Control::Use => "vis/touch/use",
        Control::Pause => "vis/touch/pause",
        Control::Inventory => "vis/touch/inventory",
        Control::Offhand => "vis/touch/offhand",
        Control::Shield => "vis/touch/custom_shield",
        _ => return None,
    })
}

impl Game<'_> {
    pub(super) fn android_touch_event(
        &mut self,
        events: &dyn EventLoopCtl,
        id: WindowId,
        touch: Touch,
        origin: input_trace::InputOrigin,
    ) {
        let playing =
            self.title.is_none() && self.screen == Screen::Playing && !self.screen_over_playing();
        if self.android_back_touch == Some(touch.id) {
            if matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.android_back_touch = None;
                if touch.phase == TouchPhase::Ended && !playing {
                    if let Err(error) = self.pad_back() {
                        log::warn!("Horizon: Android menu back: {error}");
                    }
                }
            }
            return;
        }
        if !playing
            && touch.phase == TouchPhase::Started
            && let Some(window) = &self.window
        {
            let size = window.inner_size();
            let size = [size.width, size.height];
            if back_hit(&HudBatch::new(size), touch.location, size) {
                self.android_back_touch = Some(touch.id);
                return;
            }
        }
        if playing {
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
            }
        } else {
            self.android_touch.clear();
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
        }
    }
    pub(super) fn android_touch_hud(&self, batch: &mut HudBatch) {
        if self.title.is_some() || self.screen != Screen::Playing || self.screen_over_playing() {
            draw_back(batch, &self.gui.atlas);
            return;
        }
        let (w, h) = self.gui_size();
        let atlas = &self.gui.atlas;
        for c in Control::ALL {
            if !self.android_touch.settings.visible(c) {
                continue;
            }
            let [x, y, rw, rh] = self.android_touch.settings.placement(c).rect(w / h.max(1.));
            if let Some(sprite) = icon(c) {
                batch.sprite_scaled(atlas, sprite, x * w, y * h, rw * w, rh * h, 0xffffffff);
                continue;
            }
            batch.fill(atlas, x * w, y * h, rw * w, rh * h, 0x88242631);
            let label = horizon_client_render::lang::get_or(
                &format!("vis.touch.{}", c.name().to_ascii_lowercase()),
                c.name(),
            );
            batch.text(atlas, &label, x * w + 2., y * h + 2., 0xffffffff, true);
        }
    }
}
