//! Timer manager

use alloc::collections::BTreeMap;

use crate::*;

pub struct TimerManager {
    timers: BTreeMap<Handle, Timer>,
    next_handle: u32,
}

pub struct Timer {
    value: u32,
    is_active: bool,
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
            value: 0,
            is_active: false,
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

    pub fn init(&mut self, handle: Handle, value: u32) {
        if let Some(timer) = self.timers.get_mut(&handle) {
            timer.value = value;
        }
    }

    pub fn set(&mut self, handle: Handle, timeout: u32) {
        if let Some(timer) = self.timers.get_mut(&handle) {
            timer.is_active = true;
            js_schedule_event(timeout, timer.value as i32);
        }
    }

    pub fn free(&mut self, handle: Handle) {
        if let Some(timer) = self.timers.get_mut(&handle) {
            timer.is_active = false;
        }
    }

    pub fn ack(&mut self, value: u32) {
        for (_, timer) in self.timers.iter_mut() {
            if timer.is_active && timer.value == value {
                timer.is_active = false;
                break;
            }
        }
    }
}
