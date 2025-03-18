use std::cmp::Ordering;

use crate::error::SuperLendyError;
use crate::LendyResult;
use bytemuck::{Pod, Zeroable};
use solana_program::clock::Slot;
use texture_common::math::MathError;

static_assertions::const_assert_eq!(0, std::mem::size_of::<LastUpdate>() % 8);

/// Last update state
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct LastUpdate {
    /// Last slot when updated
    pub slot: Slot,
    pub timestamp: i64,

    /// 1 - means stale state, 0 - means up to date state
    pub stale: u8,

    pub _padding: [u8; 15],
}

impl LastUpdate {
    /// Create new last update
    pub fn new(slot: Slot, timestamp: i64) -> Self {
        Self {
            slot,
            timestamp,
            stale: 1,
            _padding: Zeroable::zeroed(),
        }
    }

    /// Return slots elapsed since given slot
    pub fn slots_elapsed(&self, slot: Slot) -> LendyResult<u64> {
        let slots_elapsed = slot.checked_sub(self.slot).ok_or(MathError(format!(
            "slots_elapsed(): checked_sub {} - {}",
            slot, self.slot
        )))?;
        Ok(slots_elapsed)
    }

    pub fn seconds_elapsed(&self, current_time: i64) -> LendyResult<u64> {
        if current_time < self.timestamp {
            return Err(SuperLendyError::MathError(MathError(format!(
                "seconds_elapsed(): current_time {} is earlier then self.timestamp {}",
                current_time, self.timestamp
            ))));
        }

        let secs_elapsed = current_time
            .checked_sub(self.timestamp)
            .ok_or(MathError(format!(
                "seconds_elapsed(): checked_sub {} - {}",
                current_time, self.timestamp
            )))?;

        Ok(secs_elapsed as u64)
    }

    /// Set last update slot
    pub fn update(&mut self, slot: Slot, timestamp: i64) {
        self.slot = slot;
        self.timestamp = timestamp;
        self.stale = 0;
    }

    /// Set stale to true
    pub fn mark_stale(&mut self) {
        self.stale = 1;
    }

    /// Check if marked stale or last update timestamp is too long ago
    pub fn is_stale(
        &self,
        current_time: i64,
        timestamp_stale_threshold_sec: u64,
    ) -> LendyResult<bool> {
        Ok(self.stale == 1 || self.seconds_elapsed(current_time)? >= timestamp_stale_threshold_sec)
    }

    pub fn is_stale_by_slot(
        &self,
        current_slot: Slot,
        timestamp_stale_threshold_slots: Slot,
    ) -> LendyResult<bool> {
        Ok(self.stale == 1 || self.slots_elapsed(current_slot)? >= timestamp_stale_threshold_slots)
    }
}

impl PartialEq for LastUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot
    }
}

impl PartialOrd for LastUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.slot.partial_cmp(&other.slot)
    }
}
