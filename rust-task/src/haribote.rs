//! Haribote OS HLE kernel

use ume86::gpr::PartialRegister;
use ume86::prelude::*;
use ume86::ume::{Exception, UME};

use crate::haribote::file::{FileManager, Whence};
use crate::haribote::timer::TimerManager;
use crate::malloc::SimpleAllocator;
use crate::prelude::*;

pub mod file;
pub mod lang;
pub mod malloc;
pub mod timer;
pub mod window;

pub mod prelude {
    pub use super::*;
    pub use crate::lang::*;
    pub use crate::window::*;
}

pub struct App {
    pub state: AppState,
    pub max_step: isize,
    pub lang_mode: LangMode,
    pub title_bar_height: u32,
    pub emulator: UME,
    pub cmdline: String,

    tsc_raw: u64,
    tsc_count: u64,

    japanese_font: Vec<u8>,
    allocator: SimpleAllocator,
    files: FileManager,
    timers: TimerManager,
    windows: Vec<HariWindow>,
}

/// Generic Handle type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Handle(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppState {
    /// Running normally
    #[default]
    Running,
    /// Waiting for a key event (non-blocking)
    GetKey(bool),
    /// Waiting for a key event (blocking)
    WaitKey(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Continue,
    Exit,
    Wait(i32),
}

impl App {
    // "HDE "
    const OS_ID: u32 = 0x20454448;

    const OS_VER: u32 = 0;

    const TIMER_ID_BIAS: u32 = 0x0001_0000;

    pub fn instantiate(binary: &[u8], cmdline: &str, title_bar_height: u32) -> Option<Self> {
        let Some(hrb) = hrb::HrbExecutable::identify(&binary) else {
            return None;
        };

        let start_data = hrb.start_data as usize;
        let size_of_data = hrb.size_of_data as usize;
        let size_of_ds = hrb.size_of_ds as usize;
        let esp = hrb.esp;

        let code = binary[..start_data].to_vec().into_boxed_slice();

        let mut data = Vec::with_capacity(size_of_ds);
        // stack area
        data.resize(esp as usize, 0u8);
        // data area
        data.extend_from_slice(&binary[start_data..][..size_of_data]);
        // bss area
        data.resize(size_of_ds, 0u8);
        let data = data.into_boxed_slice();

        let emulator = UME::new(
            code,
            data,
            Offset32(0),
            Offset32(hrb.entry_point()),
            Offset32(esp),
            Box::new(|msg| {
                crate::println!("[rust] UME: {}", msg);
            }),
        );

        let japanese_font = rust_read_file("nihongo.fnt").unwrap_or_default();
        let lang_mode = if japanese_font.is_empty() {
            crate::lang::LangMode::Ascii
        } else {
            crate::lang::LangMode::ShiftJIS
        };

        Some(Self {
            state: AppState::Running,
            max_step: 1_000_000,
            lang_mode,
            title_bar_height,
            cmdline: cmdline.to_string(),
            emulator,
            allocator: SimpleAllocator::new(),
            windows: Vec::new(),
            timers: TimerManager::new(),
            files: FileManager::new(),
            tsc_raw: 0,
            tsc_count: 0,
            japanese_font,
        })
    }

    /// Returns the adjusted keycode based on the input code and whether it's an extended key.
    pub fn adjust_keycode(&mut self, code: i32, is_ex: bool) -> u32 {
        if code == -1 {
            return u32::MAX;
        }
        let code = code as u32;
        if code > Self::TIMER_ID_BIAS {
            self.timers.ack(code);
            let timer_value = code - Self::TIMER_ID_BIAS;
            timer_value
        } else if is_ex {
            code
        } else {
            let code = code & 0xff;
            let code = match code {
                0x84 => 0x34,       // Arrow Left
                0x85 => 0x36,       // Arrow Right
                0x86 => 0x38,       // Arrow Up
                0x87 => 0x32,       // Arrow Down
                0x80.. => u32::MAX, // Ignore other keys
                _ => code,
            };
            code
        }
    }

    /// Run the application
    pub fn run(&mut self) -> Result<ExitStatus, String> {
        match self.state {
            AppState::Running => {}
            AppState::GetKey(is_ex) => {
                let key = self.adjust_keycode(js_get_keyboard_event(0), is_ex);
                self.emulator.state().eax().write(key);
                self.state = AppState::Running;
            }
            AppState::WaitKey(is_ex) => {
                let key = self.adjust_keycode(js_get_keyboard_event(1), is_ex);
                if key == u32::MAX {
                    return Ok(ExitStatus::Wait(10));
                } else {
                    self.emulator.state().eax().write(key);
                    self.state = AppState::Running;
                }
            }
        }

        let status = loop {
            match self.emulator.execute() {
                Ok(()) => break Ok(ExitStatus::Continue),
                Err(Exception::Swi(64)) => match self.syscall() {
                    Ok(ExitStatus::Continue) => {
                        self.emulator.resume_next();
                        continue;
                    }
                    Ok(v) => {
                        self.emulator.resume_next();
                        break Ok(v);
                    }
                    Err(e) => break Err(e),
                },
                Err(Exception::RdTsc) => {
                    let tsc_raw = js_get_tick() as u64;
                    let tsc_delta = tsc_raw.wrapping_sub(self.tsc_raw);
                    self.tsc_raw = tsc_raw;

                    let tsc_delta = tsc_delta * self.max_step as u64;
                    let tsc = self.tsc_count.wrapping_add(tsc_delta);
                    self.tsc_count = tsc;

                    self.emulator.state().eax().write(tsc as u32);
                    self.emulator.state().edx().write((tsc >> 32) as u32);
                    self.emulator.resume_next();
                    continue;
                }
                Err(e) => break Err(e),
            };
        };

        match status {
            Ok(v) => Ok(v),
            Err(err) => {
                let mut buf = Vec::new();

                let tracer = self.emulator.tracer();
                let map = tracer.address_map();
                for (addr, upc) in map.iter() {
                    buf.push(format!("[rust] address_map: {:?} => upc={:?}", *addr, upc));
                }
                for (i, uop) in tracer.uop_cache().iter().enumerate() {
                    buf.push(format!("[rust] uop_cache[{}]: {:?}", i, uop));
                }
                let last_upc = tracer.current_upc();

                let state = self.emulator.state_mut();
                let flags = state.compute_flags();
                buf.push(format!(
                    "[rust] {:?}, EIP={:#010x}, UPC={} Flags={:#010x}",
                    err,
                    state.eip().read(),
                    last_upc.0,
                    flags.bits(),
                ));
                buf.push(format!(
                    "[rust] EAX={:#010x}, EBX={:#010x}, ECX={:#010x}, EDX={:#010x}",
                    state.eax().read(),
                    state.ebx().read(),
                    state.ecx().read(),
                    state.edx().read(),
                ));
                buf.push(format!(
                    "[rust] ESI={:#010x}, EDI={:#010x}, ESP={:#010x}, EBP={:#010x}",
                    state.esi().read(),
                    state.edi().read(),
                    state.esp().read(),
                    state.ebp().read(),
                ));

                crate::println!("{}", buf.join("\n"));
                return Err(format!("{:?}", err));
            }
        }
    }

    #[inline]
    pub fn dispose(&mut self) {
        self.windows.clear();
    }

    #[inline]
    pub fn japanese_font(&self) -> &[u8] {
        &self.japanese_font
    }

    pub fn get_cstr<'a>(&'a self, addr: u32) -> Result<JisString<'a>, Exception> {
        let data = self.emulator.data();
        let addr = addr as usize;
        if addr >= data.len() {
            return Err(Exception::SegmentationViolation(Offset32(addr as u32)));
        }
        let mut end = addr;
        while end < data.len() && data[end] != 0 {
            end += 1;
        }
        if end >= data.len() {
            return Err(Exception::SegmentationViolation(Offset32(end as u32)));
        }
        let bytes = &data[addr..end];
        Ok(JisString::from_bytes(bytes))
    }

    #[inline]
    pub fn window_handle<'a>(&'a self, handle: u32) -> Option<(&'a HariWindow, bool)> {
        let redraw = (handle & 1) == 0;
        let window_id = (handle >> 1).wrapping_sub(1) as usize;
        if self.windows.len() <= window_id as usize {
            return None;
        }
        Some((&self.windows[window_id as usize], redraw))
    }

    /// Haribote OS syscall handler
    pub fn syscall(&mut self) -> Result<ExitStatus, Exception> {
        match self.emulator.state().edx().read() {
            1 => {
                // putchar
                crate::print!("{}", self.emulator.state().al().read() as char);
            }
            2 => {
                // putstring (cstr)
                let s = self.get_cstr(self.emulator.state().ebx().read())?;
                crate::print!("{}", s);
            }
            // 3 putstring (length:ecx, string:ebx)
            4 => {
                // exit
                return Ok(ExitStatus::Exit);
            }
            5 => {
                // open_win(title:ecx, width:esi, height:edi, buffer:ebx)
                let title = self.get_cstr(self.emulator.state().ecx().read())?;
                let width = self.emulator.state().esi().read();
                let height = self.emulator.state().edi().read();
                let buffer = self.emulator.state().ebx().read();
                crate::println!(
                    "[rust] open_win({:?}, {}, {}, {:08x})",
                    title.to_str().unwrap_or(""),
                    width,
                    height,
                    buffer,
                );
                let window = HariWindow::new(self, &title, Size { width, height }, buffer);
                self.windows.push(window);
                let window_id = (self.windows.len() as u32) << 1;
                self.emulator.state().eax().write(window_id);
            }
            6 => {
                // draw_text(window:ebx, x: esi, y:edi, color: eax, text:ebp, ecx: length)
                match self.window_handle(self.emulator.state().ebx().read()) {
                    Some((window, redraw)) => {
                        let x = self.emulator.state().esi().read() as i32;
                        let y = self.emulator.state().edi().read() as i32;
                        let color = self.emulator.state().eax().read() as u8;
                        let text = self.get_cstr(self.emulator.state().ebp().read())?;
                        let max_len = self.emulator.state().ecx().read() as usize;
                        window.draw_string(self, Point::new(x, y), &text, max_len, color, redraw);
                    }
                    None => {}
                }
            }
            7 => {
                // fill_rect(window:ebx, x0:eax, y0:ecx, x1:esi, y1:edi, color:ebp)
                self.window_handle(self.emulator.state().ebx().read())
                    .map(|(window, redraw)| {
                        let x0 = self.emulator.state().eax().read() as i32;
                        let y0 = self.emulator.state().ecx().read() as i32;
                        let x1 = self.emulator.state().esi().read() as i32;
                        let y1 = self.emulator.state().edi().read() as i32;
                        let color = self.emulator.state().ebp().read() as u8;
                        window.fill_rect(
                            self,
                            Point::new(x0, y0),
                            Point::new(x1, y1),
                            color,
                            redraw,
                        );
                    });
            }
            8 => {
                // init malloc (start:eax, size:ecx)
                let start = self.emulator.state().eax().read() as usize;
                let size = self.emulator.state().ecx().read() as usize;
                self.allocator.init(start, size);
            }
            9 => {
                // malloc (size:ecx)
                let size = self.emulator.state().ecx().read() as usize;
                let addr = self.allocator.alloc(size);
                self.emulator.state().eax().write(addr.unwrap_or(0) as u32);
            }
            10 => {
                // free (ptr:eax, size:ecx)
                let addr = self.emulator.state().eax().read() as usize;
                let size = self.emulator.state().ecx().read() as usize;
                self.allocator.free(addr, size);
            }
            11 => {
                // set pixel (window:ebx, x:esi, y:edi, color:al)
                self.window_handle(self.emulator.state().ebx().read())
                    .map(|(window, redraw)| {
                        let x = self.emulator.state().esi().read() as i32;
                        let y = self.emulator.state().edi().read() as i32;
                        let color = self.emulator.state().al().read() as u8;
                        window.set_pixel(self, Point::new(x, y), color, redraw);
                    });
            }
            12 => {
                // refresh window (window:ebx, x0:eax, y0:ecx, x1:esi, y1:edi)
                self.window_handle(self.emulator.state().ebx().read())
                    .map(|(window, _redraw)| {
                        let x0 = self.emulator.state().eax().read() as i32;
                        let y0 = self.emulator.state().ecx().read() as i32;
                        let x1 = self.emulator.state().esi().read() as i32;
                        let y1 = self.emulator.state().edi().read() as i32;
                        window.redraw_rect(self, Point::new(x0, y0), Point::new(x1, y1));
                    });
            }
            13 => {
                // draw_line(window:ebx, x0:eax, y0:ecx, x1:esi, y1:edi, color:ebp)
                self.window_handle(self.emulator.state().ebx().read())
                    .map(|(window, redraw)| {
                        let x0 = self.emulator.state().eax().read() as i32;
                        let y0 = self.emulator.state().ecx().read() as i32;
                        let x1 = self.emulator.state().esi().read() as i32;
                        let y1 = self.emulator.state().edi().read() as i32;
                        let color = self.emulator.state().ebp().read() as u8;
                        window.draw_line(
                            self,
                            Point::new(x0, y0),
                            Point::new(x1, y1),
                            color,
                            redraw,
                        );
                    });
            }
            14 => {
                // TODO: close window
            }
            15 => {
                // get key
                let sleep = self.emulator.state().eax().read() != 0;
                if sleep {
                    self.state = AppState::WaitKey(false);
                    return Ok(ExitStatus::Wait(10));
                } else {
                    self.state = AppState::GetKey(false);
                    return Ok(ExitStatus::Wait(0));
                }
            }
            16 => {
                // alloc timer
                self.emulator.state().eax().write(self.timers.allocate().0);
            }
            17 => {
                // init timer (handle:ebx, value:eax)
                self.timers.init(
                    Handle(self.emulator.state().ebx().read()),
                    self.emulator.state().eax().read() + Self::TIMER_ID_BIAS,
                );
            }
            18 => {
                // set timer (handle:ebx, timeout:eax)
                self.timers.set(
                    Handle(self.emulator.state().ebx().read()),
                    self.emulator.state().eax().read() * 10, /* 10ms */
                );
            }
            19 => {
                // free timer (handle:ebx)
                self.timers.free(Handle(self.emulator.state().ebx().read()));
            }
            20 => {
                // beep
                js_play_sound(self.emulator.state().eax().read());
            }
            21 => {
                // open file for read
                let filename = self.get_cstr(self.emulator.state().ebx().read())?;
                let filename_str = filename.to_str().unwrap_or("").to_owned();
                self.emulator.state().eax().write(
                    self.files
                        .open_file(&filename_str)
                        .map(|handle| handle.0)
                        .unwrap_or(0),
                );
            }
            22 => {
                // close file
                let handle = Handle(self.emulator.state().eax().read());
                self.files.close_file(handle);
            }
            23 => {
                // seek file
                let handle = Handle(self.emulator.state().eax().read());
                self.files.get(handle).map(|file| {
                    let offset = self.emulator.state().ebx().read() as isize;
                    let whence =
                        Whence::from_u32(self.emulator.state().ecx().read()).unwrap_or_default();
                    file.seek(offset, whence);
                });
            }
            24 => {
                // get file size
                let handle = Handle(self.emulator.state().eax().read());
                self.files.get(handle).map(|file| {
                    let whence =
                        Whence::from_u32(self.emulator.state().ecx().read()).unwrap_or_default();
                    let size = file.get_file_size(whence);
                    self.emulator.state().eax().write(size as u32);
                });
            }
            25 => {
                // read file
                let handle = Handle(self.emulator.state().eax().read());

                let buf_ptr = self.emulator.state().ebx().read() as usize;
                let max_len = self.emulator.state().ecx().read() as usize;
                let data = self.emulator.data();
                if buf_ptr >= data.len() || buf_ptr.saturating_add(max_len) > data.len() {
                    return Err(Exception::SegmentationViolation(Offset32(buf_ptr as u32)));
                }

                let read = self
                    .files
                    .get(handle)
                    .map(|file| file.read(max_len))
                    .map(|read_data| {
                        data[buf_ptr..buf_ptr + read_data.len()].copy_from_slice(&read_data);
                        read_data.len()
                    })
                    .unwrap_or(0);
                self.emulator.state().eax().write(read as u32);
            }
            26 => {
                // get cmdline
                let cmdline = self.cmdline.as_bytes();
                let buf_ptr = self.emulator.state().ebx().read() as usize;
                let buf_len = self.emulator.state().ecx().read() as usize;
                let data = self.emulator.data();
                let end = buf_ptr.saturating_add(buf_len);
                if buf_ptr >= data.len() || end > data.len() {
                    return Err(Exception::SegmentationViolation(Offset32(buf_ptr as u32)));
                }
                data[buf_ptr..buf_ptr + buf_len].fill(0);
                let copy_len = core::cmp::min(buf_len, cmdline.len());
                data[buf_ptr..buf_ptr + copy_len].copy_from_slice(&cmdline[..copy_len]);
                self.emulator.state().eax().write(copy_len as u32);
            }
            27 => {
                // get lang mode
                self.emulator.state().eax().write(self.lang_mode as u32);
                self.emulator.state().ecx().write(Self::OS_ID);
                self.emulator.state().edx().write(Self::OS_VER);
            }
            28 => {
                // open file for write (filename:ebx) -> handle:eax
                // TODO: not implemented
                self.emulator.state().eax().write(0);
            }
            29 => {
                // write file (handle:eax, buffer:ebx, length:ecx) -> written:eax
                // TODO: not implemented
                self.emulator.state().eax().write(0);
            }
            // 30 => {
            //     // void api_osselect(int i);
            //     // This API is not supported due to architectural differences.
            // }
            // 31 => {
            //     // int api_sendkey(char *);
            //     // This API is not supported due to architectural differences.
            // }
            // 32 => {
            //     // void api_semiFlat(void);
            //     // This API is not supported due to architectural differences.
            // }
            33 => {
                // extended API 33
                match self.emulator.state().ecx().read() {
                    1 => {
                        // int api_getTimeCount(void);
                        let timer = self.timers.get_monotonic_timer();
                        self.emulator.state().eax().write((timer / 10.0) as u32);
                    }
                    2 => {
                        // int api_getkeyEx(int mode);
                        let sleep = self.emulator.state().eax().read() != 0;
                        if sleep {
                            self.state = AppState::WaitKey(true);
                            return Ok(ExitStatus::Wait(10));
                        } else {
                            self.state = AppState::GetKey(true);
                            return Ok(ExitStatus::Wait(0));
                        }
                    }
                    _ => self.emulator.state().eax().write(0),
                }
            }
            value => {
                crate::println!("Unsupported system call: EDX={:#010x}", value);
                return Ok(ExitStatus::Exit);
            }
        }
        Ok(ExitStatus::Continue)
    }
}
