use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Atomic capabilities supported across AI providers and models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    TextGeneration,
    Streaming,
    ToolCalling,
    Vision,
    RealtimeAudio,
    SpeechToText,
    TextToSpeech,
    Embeddings,
}

/// A typed set of capabilities.
/// Distinguishes between provider capabilities, model capabilities,
/// and computes effective capability intersections.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    capabilities: HashSet<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            capabilities: HashSet::new(),
        }
    }

    pub fn from_slice(caps: &[Capability]) -> Self {
        let mut set = HashSet::with_capacity(caps.len());
        for cap in caps {
            set.insert(*cap);
        }
        Self { capabilities: set }
    }

    pub fn has(&self, cap: Capability) -> bool {
        self.capabilities.contains(&cap)
    }

    pub fn insert(&mut self, cap: Capability) -> bool {
        self.capabilities.insert(cap)
    }

    pub fn remove(&mut self, cap: Capability) -> bool {
        self.capabilities.remove(&cap)
    }

    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.capabilities.iter()
    }

    /// Computes effective capabilities by intersecting provider capabilities
    /// with model capabilities.
    ///
    /// Example: Provider supports RealtimeAudio, but Model only supports Text + Streaming.
    /// Effective capability = Text + Streaming (RealtimeAudio unavailable).
    pub fn intersection(&self, other: &CapabilitySet) -> CapabilitySet {
        let set = self
            .capabilities
            .intersection(&other.capabilities)
            .copied()
            .collect();
        CapabilitySet { capabilities: set }
    }

    /// Returns capabilities present in `self` but not in `other`.
    pub fn difference(&self, other: &CapabilitySet) -> CapabilitySet {
        let set = self
            .capabilities
            .difference(&other.capabilities)
            .copied()
            .collect();
        CapabilitySet { capabilities: set }
    }

    /// Extends this capability set with all items from an iterator.
    pub fn extend<I: IntoIterator<Item = Capability>>(&mut self, iter: I) {
        self.capabilities.extend(iter);
    }
}

impl FromIterator<Capability> for CapabilitySet {
    fn from_iter<T: IntoIterator<Item = Capability>>(iter: T) -> Self {
        Self {
            capabilities: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_set_basic() {
        let mut set = CapabilitySet::new();
        assert!(!set.has(Capability::TextGeneration));
        set.insert(Capability::TextGeneration);
        assert!(set.has(Capability::TextGeneration));
        assert_eq!(set.len(), 1);

        set.insert(Capability::Streaming);
        assert!(set.has(Capability::Streaming));
        assert_eq!(set.len(), 2);

        set.remove(Capability::TextGeneration);
        assert!(!set.has(Capability::TextGeneration));
        assert!(set.has(Capability::Streaming));
    }

    #[test]
    fn test_effective_capability_intersection() {
        // Provider supports Text, Streaming, ToolCalling, RealtimeAudio
        let provider_caps = CapabilitySet::from_slice(&[
            Capability::TextGeneration,
            Capability::Streaming,
            Capability::ToolCalling,
            Capability::RealtimeAudio,
        ]);

        // Model supports Text, Streaming, Vision (but not RealtimeAudio or ToolCalling)
        let model_caps = CapabilitySet::from_slice(&[
            Capability::TextGeneration,
            Capability::Streaming,
            Capability::Vision,
        ]);

        // Effective capabilities should be Text + Streaming
        let effective = provider_caps.intersection(&model_caps);
        assert!(effective.has(Capability::TextGeneration));
        assert!(effective.has(Capability::Streaming));
        assert!(!effective.has(Capability::RealtimeAudio));
        assert!(!effective.has(Capability::ToolCalling));
        assert!(!effective.has(Capability::Vision));
        assert_eq!(effective.len(), 2);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let set = CapabilitySet::from_slice(&[Capability::TextGeneration, Capability::Streaming]);
        let json = serde_json::to_string(&set).unwrap();
        let deserialized: CapabilitySet = serde_json::from_str(&json).unwrap();
        assert_eq!(set, deserialized);
    }
}
