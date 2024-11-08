//! Branch history table.

use super::*;

/// 2-bit saturation counter.
#[derive(Debug, Default, Clone, Copy)]
pub enum SatCounter {
    /// Strongly not taken.
    StronglyNotTaken,

    /// Weakly not taken.
    #[default]
    WeaklyNotTaken,

    /// Weakly taken.
    WeaklyTaken,

    /// Strongly taken.
    StronglyTaken,
}

impl SatCounter {
    /// Increments the counter.
    pub fn increment(self) -> Self {
        match self {
            SatCounter::StronglyNotTaken => SatCounter::WeaklyNotTaken,
            SatCounter::WeaklyNotTaken => SatCounter::WeaklyTaken,
            SatCounter::WeaklyTaken => SatCounter::StronglyTaken,
            SatCounter::StronglyTaken => SatCounter::StronglyTaken,
        }
    }

    /// Decrements the counter.
    pub fn decrement(self) -> Self {
        match self {
            SatCounter::StronglyNotTaken => SatCounter::StronglyNotTaken,
            SatCounter::WeaklyNotTaken => SatCounter::StronglyNotTaken,
            SatCounter::WeaklyTaken => SatCounter::WeaklyNotTaken,
            SatCounter::StronglyTaken => SatCounter::WeaklyTaken,
        }
    }

    /// Predicts the branch is taken or not.
    pub fn predict(self) -> bool {
        match self {
            SatCounter::StronglyNotTaken | SatCounter::WeaklyNotTaken => false,
            SatCounter::WeaklyTaken | SatCounter::StronglyTaken => true,
        }
    }
}

/// BHT.
#[derive(Debug, Default, Clone, Copy)]
pub struct Bht {
    /// BHT entries.
    #[allow(unused)]
    pub entries: Array<SatCounter, BHT_ENTRIES>,
}

impl Bht {
    /// Predicts the direction of a branch instruction with the given PC.
    ///
    /// Returns `true` if the branch is prediction as taken; otherwise, returns `false`.
    pub fn predict(self, _pc: u32) -> bool {
        let index = (_pc as usize) % BHT_ENTRIES;
        let counter = self.entries[index];
        counter.predict()
    }

    /// Returns the updated BHT when a branch instruction resolves at the execute stage with the given PC.
    ///
    /// It updates the entry corresponding to the given PC.
    pub fn update(self, _pc: u32, _taken: bool) -> Self {
        let index = (_pc as usize) % BHT_ENTRIES;
        let counter = self.entries[index];

        let new_counter = if _taken {
            counter.increment()
        } else {
            counter.decrement()
        };
        
        Bht{    
            entries: self.entries.set(index, new_counter),
        }
    }
}
