//! Android load limits. Runtime caps never overwrite the player's saved settings.
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    pub distance: i32,
    pub scale: u32,
    pub textures_mib: u32,
    pub uploads_kib: u32,
}
impl Profile {
    /// Conservative first-run defaults; unknown RAM uses the smallest tier.
    pub fn for_memory(bytes: u64, low_ram: bool) -> Self {
        if low_ram || bytes < 4 * 1024 * 1024 * 1024 {
            Self {
                distance: 4,
                scale: 65,
                textures_mib: 256,
                uploads_kib: 1024,
            }
        } else if bytes < 8 * 1024 * 1024 * 1024 {
            Self {
                distance: 6,
                scale: 75,
                textures_mib: 512,
                uploads_kib: 2048,
            }
        } else {
            Self {
                distance: 8,
                scale: 85,
                textures_mib: 768,
                uploads_kib: 4096,
            }
        }
    }
}

#[derive(Default)]
pub struct Governor {
    level: u8,
    cooling_since: Option<Instant>,
}
impl Governor {
    /// Android thermal status: 0 none, 1 light, 2 moderate, 3 severe, 4+ critical.
    /// Missing/invalid readings do not undo a known thermal restriction.
    pub fn observe(&mut self, status: Option<i32>, now: Instant) {
        let Some(status @ 0..=6) = status else { return };
        let desired = match status {
            0..=1 => 0,
            2 => 1,
            3 => 2,
            _ => 3,
        };
        if desired >= self.level {
            self.level = desired;
            self.cooling_since = None;
        } else {
            let since = *self.cooling_since.get_or_insert(now);
            if now.saturating_duration_since(since) >= Duration::from_secs(30) {
                self.level -= 1;
                self.cooling_since = None;
            }
        }
    }
    pub fn cap(&self, requested: i32) -> i32 {
        requested.min(match self.level {
            0 => i32::MAX,
            1 => 60,
            2 => 45,
            _ => 30,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn heat_limits_immediately_and_recovers_slowly_without_raising_user_cap() {
        let now = Instant::now();
        let mut g = Governor::default();
        g.observe(Some(4), now);
        assert_eq!(g.cap(120), 30);
        assert_eq!(g.cap(20), 20);
        g.observe(Some(0), now);
        g.observe(None, now + Duration::from_secs(31));
        assert_eq!(g.cap(120), 30);
        g.observe(Some(0), now + Duration::from_secs(31));
        assert_eq!(g.cap(120), 45);
        g.observe(Some(3), now + Duration::from_secs(32));
        assert_eq!(g.cap(120), 45);
    }
    #[test]
    fn unknown_or_low_ram_uses_conservative_profile() {
        assert_eq!(Profile::for_memory(0, false).distance, 4);
        assert_eq!(Profile::for_memory(u64::MAX, true).textures_mib, 256);
        assert_eq!(Profile::for_memory(6 << 30, false).distance, 6);
        assert_eq!(Profile::for_memory(8 << 30, false).distance, 8);
    }
}
