//! HLE Haribote OS Task Runner

// #![cfg_attr(not(test), no_std)]
#![feature(asm_experimental_arch)]

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use wasm_bindgen::prelude::*;

use crate::haribote::prelude::*;

extern crate alloc;

pub mod haribote;

pub mod prelude {
    pub use super::*;
}

#[wasm_bindgen(module = "env")]
unsafe extern "C" {
    fn js_open_window(width: u32, height: u32, title_ptr: *const u8, title_len: u32) -> u32;
    fn js_move_window(window_id: u32, x: i32, y: i32);
    fn js_activate_window(window_id: u32);
    fn js_close_window(window_id: u32);
    fn js_draw_image(
        window_id: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        ptr: *const u8,
        len: u32,
    );
    fn js_print(text_ptr: *const u8, text_len: u32);
    fn js_read_file_size(filename_ptr: *const u8, filename_len: u32) -> i32;
    fn js_read_file_into(buf_ptr: *mut u8, buf_len: u32) -> i32;
    fn js_write_file(
        filename_ptr: *const u8,
        filename_len: u32,
        data_ptr: *const u8,
        data_len: u32,
        mode: u32,
    ) -> i32;
    fn js_get_keyboard_event(window_id: u32) -> i32;
    fn js_get_tick() -> f64;
    fn js_schedule_event(delay_ms: u32, event_code: i32);
    fn js_play_sound(frequency: u32);
}

pub struct StdOut;

impl core::fmt::Write for StdOut {
    #[inline]
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        js_print(s.as_ptr(), s.len() as u32);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        #[allow(unused_imports)]
        use core::fmt::Write;
        let _ = write!(StdOut, $($arg)*);
    }};
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        #[allow(unused_imports)]
        use core::fmt::Write;
        let _ = writeln!(StdOut, $($arg)*);
    }};
}

/// Custom panic handler that prints the panic information and halts execution.
fn panic2(info: &std::panic::PanicHookInfo) -> ! {
    println!("{}", info);

    unsafe {
        core::arch::asm!("unreachable", options(noreturn));
    }
}

/// Write mode for `rust_write_file`.
#[repr(u32)]
#[allow(dead_code)]
#[derive(Clone, Copy)]
enum WriteMode {
    /// Update existing file; error if not found.
    Update = 0,
    /// Create new file; error if already exists.
    Create = 1,
    /// Overwrite if exists, create if not.
    Upsert = 2,
}

/// Read a file from the filesystem. Returns `None` if not found.
#[allow(dead_code)]
fn rust_read_file(filename: &str) -> Option<Vec<u8>> {
    let size = js_read_file_size(filename.as_ptr(), filename.len() as u32);
    if size < 0 {
        return None;
    }
    let mut buf = Vec::new();
    buf.resize(size as usize, 0u8);
    if size > 0 {
        let read = js_read_file_into(buf.as_mut_ptr(), size as u32);
        if read < 0 {
            return None;
        }
    }
    Some(buf)
}

/// Write data to a file with the given mode. Returns `true` on success.
#[allow(dead_code)]
fn rust_write_file(filename: &str, data: &[u8], mode: WriteMode) -> bool {
    js_write_file(
        filename.as_ptr(),
        filename.len() as u32,
        data.as_ptr(),
        data.len() as u32,
        mode as u32,
    ) == 0
}

/// Application instance
static mut APP: UnsafeCell<Option<App>> = UnsafeCell::new(None);

/// Run task with parameters from the Worker
#[wasm_bindgen]
pub fn run_task(file_name: String, cmdline: String, title_bar_height: u32) {
    std::panic::set_hook(Box::new(|info| {
        panic2(info);
    }));

    println!(
        "[rust] run_task({:#?}, {:#?}, {:#?})",
        file_name, cmdline, title_bar_height
    );

    let mut binary = rust_read_file(&file_name).expect("Failed to read binary file");

    // Decompress tek
    if let Ok(_) = tek::tek_getsize(&binary) {
        match tek::tek_decomp(&binary) {
            Ok(v) => binary = v,
            Err(err) => {
                println!("[tek] Failed to decompress binary: {:?}", err);
                return;
            }
        };
    }

    let Some(app) = App::instantiate(&binary, &cmdline, title_bar_height) else {
        println!("Bad executable");
        return;
    };

    // Safety: We ensure that APP is only accessed in a single-threaded context.
    unsafe {
        *(&mut *(&raw mut APP)).get_mut() = Some(app);
    }
}

#[wasm_bindgen]
pub fn r#loop(speed: isize) -> Result<i32, String> {
    // Safety: We ensure that APP is only accessed in a single-threaded context.
    let app = unsafe { (&mut *(&raw mut APP)).get_mut() };
    if let Some(app) = app {
        match app.run(speed) {
            Ok(status) => match status {
                ExitStatus::Continue => Ok(0),
                ExitStatus::Exit => {
                    app.dispose();
                    println!("[rust] run_task end");
                    Ok(-1)
                }
                ExitStatus::Wait(code) => Ok(code),
            },
            Err(err) => {
                app.dispose();
                Err(err)
            }
        }
    } else {
        Ok(-1)
    }
}
