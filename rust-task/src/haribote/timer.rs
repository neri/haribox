//! Timer manager

use alloc::collections::BTreeMap;

use crate::*;

pub struct TimerManager {
    timers: BTreeMap<Handle, Timer>,
    next_handle: u32,
}

pub struct Timer {
    /// Data delivered to the application when the timer expires
    data: u32,
    /// Expiration time in milliseconds of the monotonic timer, `None` if not running
    deadline: Option<f64>,
}

impl TimerManager {
    #[inline]
    pub const fn new() -> Self {
        Self {
            timers: BTreeMap::new(),
            next_handle: 1,
        }
    }

    pub fn allocate(&mut self) -> Handle {
        let timer = Timer {
            data: 0,
            deadline: None,
        };
        let handle = Handle(self.next_handle);
        self.next_handle += 1;
        self.timers.insert(handle, timer);
        handle
    }

    #[inline]
    pub fn get_monotonic_timer(&self) -> f64 {
        js_get_tick()
    }

    pub fn init(&mut self, handle: Handle, data: u32) {
        if let Some(timer) = self.timers.get_mut(&handle) {
            timer.data = data;
        }
    }

    /// Starts the timer. If it is already running, the previous timeout is replaced.
    pub fn set(&mut self, handle: Handle, timeout: u32) {
        let now = self.get_monotonic_timer();
        if let Some(timer) = self.timers.get_mut(&handle) {
            timer.deadline = Some(now + timeout as f64);
        }
    }

    /// Stops the timer. An expiration that has not been delivered yet is cancelled.
    pub fn free(&mut self, handle: Handle) {
        if let Some(timer) = self.timers.get_mut(&handle) {
            timer.deadline = None;
        }
    }

    /// Stops all expired timers and calls `f` with the data of each one, in order of expiration.
    pub fn take_expired(&mut self, mut f: impl FnMut(u32)) {
        let now = self.get_monotonic_timer();
        let mut expired = Vec::new();
        for timer in self.timers.values_mut() {
            if let Some(deadline) = timer.deadline
                && deadline <= now
            {
                timer.deadline = None;
                expired.push((deadline, timer.data));
            }
        }
        expired.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, data) in expired {
            f(data);
        }
    }

    /// Returns the time in milliseconds until the next timer expires, limited to `max`.
    pub fn time_to_next(&self, max: u32) -> u32 {
        let now = self.get_monotonic_timer();
        self.timers
            .values()
            .filter_map(|timer| timer.deadline)
            .map(|deadline| (deadline - now).ceil().max(0.0) as u32)
            .fold(max, u32::min)
    }
}
