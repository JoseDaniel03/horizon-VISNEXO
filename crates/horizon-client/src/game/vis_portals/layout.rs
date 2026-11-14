//! One geometry source for pointer hit testing and rendering.
#[derive(Clone, Copy, Debug)]
pub(super) struct Panel {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub left: f32,
}
impl Panel {
    pub fn new(w: f32, h: f32, origin: Option<(f32, f32)>) -> Self {
        let width = (w - 16.).clamp(1., 560.);
        let height = (h - 16.).max(1.);
        let (x, y) = origin.unwrap_or(((w - width) / 2., 8.));
        Self {
            x: x.clamp(0., (w - width).max(0.)),
            y: y.clamp(0., (h - height).max(0.)),
            w: width,
            h: height,
            left: (width * 0.36).min(180.),
        }
    }
    pub fn compact(self) -> bool {
        self.w < 280. || self.h < 210.
    }
    pub fn top(self, i: usize) -> [f32; 4] {
        [
            self.x + 8. + i as f32 * (self.w - 16.) / 4.,
            self.y + 36.,
            (self.w - 20.) / 4.,
            18.,
        ]
    }
    pub fn action(self, i: usize) -> [f32; 4] {
        [
            self.x + 8. + (i % 3) as f32 * (self.w - 16.) / 3.,
            self.y + self.h - 66. - (1 - i / 3) as f32 * 22.,
            (self.w - 20.) / 3.,
            18.,
        ]
    }
    pub fn row(self, i: usize) -> [f32; 4] {
        [
            self.x + 8.,
            self.y + 74. + i as f32 * 25.,
            self.left - 16.,
            23.,
        ]
    }
    pub fn rows(self) -> usize {
        ((self.h - 174.) / 25.).max(0.) as usize
    }
    pub fn chat_x(self) -> f32 {
        self.x + self.left + 10.
    }
    pub fn content_bottom(self) -> f32 {
        self.y + self.h - 118.
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moved_panel_keeps_controls_in_the_same_place() {
        let p = Panel::new(800., 340., Some((87., 16.)));
        assert_eq!(p.top(0)[1], 52.);
        for i in 0..6 {
            let [x, y, w, h] = p.action(i);
            assert!(x >= p.x && y >= p.y);
            assert!(x + w <= p.x + p.w && y + h <= p.y + p.h);
        }
    }
    #[test]
    fn narrow_panel_never_extends_outside_viewport() {
        for (w, h) in [(160., 120.), (320., 240.), (1920., 1080.)] {
            let p = Panel::new(w, h, Some((900., 900.)));
            assert!(p.x + p.w <= w && p.y + p.h <= h);
        }
    }
}
