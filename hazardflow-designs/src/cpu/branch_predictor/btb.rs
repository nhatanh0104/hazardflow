//! Branch target buffer.

use super::*;

/// BTB.
#[derive(Debug, Default, Clone, Copy)]
pub struct Btb {
    /// BTB entries.
    #[allow(unused)]
    pub entries: Array<HOption<u32>, BTB_ENTRIES>,
}

impl Btb {
    /// Returns the predicted target address of a JALR instruction with the given PC.
    pub fn predict(self, _pc: u32) -> HOption<u32> {
        let index = (_pc as usize) % BTB_ENTRIES;
        self.entries[index]
    }

    /// Returns the updated BTB when a target address misprediction occurs.
    ///
    /// It updates the entry corresponding to the given PC with the given correct target address.
    pub fn update(self, _pc: u32, _target: u32) -> Self {
        let index = (_pc as usize) % BTB_ENTRIES;
        let new_entry = Some(_target);

        Btb {
            entries: self.entries.set(index, new_entry) 
        }
    }
}
