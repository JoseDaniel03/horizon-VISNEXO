//! Android translates touch into existing controller/mouse paths; simulation stays shared.
use super::*;
use horizon_client::touch::{TouchControls, layout::{Control, HIDDEN}};
use winit::event::{Touch, TouchPhase};
pub(super) fn controls() -> TouchControls {
    let mut controls=TouchControls::default();
    for c in [Control::Chat,Control::Settings,Control::Debug,Control::Perspective] {
        controls.settings.button_visibility[c as usize]=HIDDEN;
    }
    controls
}
impl Game<'_> {
    pub(super) fn android_touch_event(&mut self, events:&dyn EventLoopCtl, id:WindowId, touch:Touch, origin:input_trace::InputOrigin) {
        if self.title.is_none() && self.screen==Screen::Playing && !self.screen_over_playing() {
            if let Some(window)=&self.window {
                let size=window.inner_size();
                self.android_touch.viewport(size.width,size.height);
                let delta=self.android_touch.update(touch.id,[touch.location.x as f32/size.width.max(1) as f32,touch.location.y as f32/size.height.max(1) as f32],touch.phase==TouchPhase::Started,matches!(touch.phase,TouchPhase::Ended|TouchPhase::Cancelled));
                let delta=horizon_client::touch::camera_delta(delta,size.width,size.height,&self.android_touch.settings);
                self.look_turn(delta[0],delta[1]);
            }
        } else {
            self.android_touch.clear();
            self.handle_window_event(events,id,WindowEvent::CursorMoved{device_id:touch.device_id,position:touch.location},origin);
            if touch.phase!=TouchPhase::Moved {
                self.handle_window_event(events,id,WindowEvent::MouseInput{device_id:touch.device_id,state:if touch.phase==TouchPhase::Started{ElementState::Pressed}else{ElementState::Released},button:MouseButton::Left},origin);
            }
        }
    }
    pub(super) fn android_touch_hud(&self,batch:&mut HudBatch) {
        if self.title.is_some() || self.screen!=Screen::Playing || self.screen_over_playing(){return;}
        let (w,h)=self.gui_size();
        let atlas=&self.gui.atlas;
        for c in Control::ALL {
            if !self.android_touch.settings.visible(c){continue;}
            let [x,y,rw,rh]=self.android_touch.settings.placement(c).rect(w/h.max(1.));
            batch.fill(atlas,x*w,y*h,rw*w,rh*h,0x66333333);
            let label=lang::get_or(&format!("vis.touch.{}",c.name().to_ascii_lowercase()),c.name());
            batch.text(atlas,&label,x*w+2.,y*h+2.,0xffffffff,true);
        }
    }
}
