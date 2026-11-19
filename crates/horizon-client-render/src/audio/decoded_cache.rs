//! Decoded-sound ownership and eviction, independent of the audio device.
use std::{collections::HashMap, sync::Arc};
/// Decoded interleaved samples, their rate and channel count (`None`: the file did not load).
pub(crate) type Samples = Option<(Arc<Vec<f32>>, u32, u16)>;
/// Decoded static sounds (`SoundBufferLibrary`'s buffers), dropped least
/// recently used first past `CACHE_BYTES`: kept forever, every sound heard in
/// a session stayed (up to 1.4 GiB of samples, villages playing hundreds).
#[derive(Default)]
pub(crate) struct SoundCache {
    pub(crate) entries: HashMap<String, (Samples, u64)>,
    pub(crate) bytes: usize,
    clock: u64,
}
pub(crate) const CACHE_BYTES: usize = 64 << 20;
impl SoundCache {
    pub(crate) fn trim(&mut self) {
        self.entries.clear();
        self.entries.shrink_to_fit();
        self.bytes = 0;
    }
    pub(crate) fn get(&mut self, key: &str) -> Option<Samples> {
        self.clock += 1;
        let entry = self.entries.get_mut(key)?;
        entry.1 = self.clock;
        Some(entry.0.clone())
    }
    pub(crate) fn insert(&mut self, key: String, samples: Samples) {
        self.clock += 1;
        self.bytes += Self::size(&samples);
        if let Some((old, _)) = self.entries.insert(key, (samples, self.clock)) {
            self.bytes -= Self::size(&old);
        }
        while self.bytes > CACHE_BYTES && self.entries.len() > 1 {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            if let Some((gone, _)) = self.entries.remove(&oldest) {
                self.bytes -= Self::size(&gone);
            }
        }
    }
    fn size(samples: &Samples) -> usize {
        samples
            .as_ref()
            .map_or(0, |(s, _, _)| s.len() * std::mem::size_of::<f32>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decoded_sounds_stay_within_the_budget_least_recently_used_out() {
        let mut cache = SoundCache::default();
        let quarter = CACHE_BYTES / 4 / std::mem::size_of::<f32>();
        let sound = || Some((Arc::new(vec![0.0f32; quarter]), 44100, 1));
        for key in ["a", "b", "c", "d"] {
            cache.insert(key.into(), sound());
        }
        assert!(cache.get("a").is_some());
        cache.insert("e".into(), sound());
        assert!(cache.bytes <= CACHE_BYTES);
        assert!(
            cache.get("b").is_none(),
            "the least recently used goes first"
        );
        assert!(cache.get("a").is_some() && cache.get("e").is_some());
        // A failed load is remembered without counting.
        cache.insert("missing".into(), None);
        assert_eq!(cache.get("missing"), Some(None));
    }
    #[test]
    fn memory_trim_keeps_playing_samples_alive_but_forgets_cache_entries() {
        let mut cache = SoundCache::default();
        let samples = Arc::new(vec![0.5f32; 32]);
        cache.insert("playing".into(), Some((samples.clone(), 44100, 1)));
        cache.trim();
        assert!(cache.entries.is_empty());
        assert_eq!(cache.bytes, 0);
        assert_eq!(samples[0], 0.5);
        assert_eq!(Arc::strong_count(&samples), 1);
    }
}
