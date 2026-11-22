//! Who switched social spy on.
use std::collections::BTreeSet;

/// A player: the two halves of his UUID.
pub type Id = (u64, u64);
/// The most social spies at once (the contract's bound).
pub const MAX_SPIES: usize = 64;

#[derive(Debug, PartialEq, Eq)]
pub enum Toggle {
    On,
    Off,
    /// Too many staff members spy already.
    Full,
}

#[derive(Default)]
pub struct Spies(BTreeSet<Id>);

impl Spies {
    pub fn toggle(&mut self, id: Id) -> Toggle {
        if self.0.remove(&id) {
            Toggle::Off
        } else if self.0.len() >= MAX_SPIES {
            Toggle::Full
        } else {
            self.0.insert(id);
            Toggle::On
        }
    }

    /// The staff member left: his switch is off.
    pub fn forget(&mut self, id: Id) {
        self.0.remove(&id);
    }

    pub fn list(&self) -> Vec<Id> {
        self.0.iter().copied().collect()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn social_spy_toggles_and_is_bounded() {
        let mut spies = Spies::default();
        assert_eq!(spies.toggle((1, 1)), Toggle::On);
        assert_eq!(spies.list(), vec![(1, 1)]);
        assert_eq!(spies.toggle((1, 1)), Toggle::Off);
        assert!(spies.list().is_empty());
        for n in 0..MAX_SPIES as u64 {
            assert_eq!(spies.toggle((n, 0)), Toggle::On);
        }
        assert_eq!(spies.toggle((u64::MAX, 0)), Toggle::Full);
        assert_eq!(spies.toggle((0, 0)), Toggle::Off);
        assert_eq!(spies.toggle((u64::MAX, 0)), Toggle::On);
        // A staff member who left frees his place.
        spies.forget((1, 0));
        spies.forget((1, 0));
        assert_eq!(spies.list().len(), MAX_SPIES - 1);
        assert_eq!(spies.toggle((1, 0)), Toggle::On);
        spies.clear();
        assert!(spies.list().is_empty());
    }
}
