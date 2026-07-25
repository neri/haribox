//! Window Wrapper for Haribote OS

use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::ops::Add;

use crate::*;

#[allow(dead_code)]
mod fonts {
    include!("hankaku.rs");
}

const PALETTE: [u32; 256] = {
    let mut palette: [u32; 256] = [0; 256];

    let mut i = 0;
    let palette16: [(u8, u8, u8); 16] = [
        (0x00, 0x00, 0x00),
        (0xff, 0x00, 0x00),
        (0x00, 0xff, 0x00),
        (0xff, 0xff, 0x00),
        (0x00, 0x00, 0xff),
        (0xff, 0x00, 0xff),
        (0x00, 0xff, 0xff),
        (0xff, 0xff, 0xff),
        (0xc6, 0xc6, 0xc6),
        (0x84, 0x00, 0x00),
        (0x00, 0x84, 0x00),
        (0x84, 0x84, 0x00),
        (0x00, 0x00, 0x84),
        (0x84, 0x00, 0x84),
        (0x00, 0x84, 0x84),
        (0x84, 0x84, 0x84),
    ];
    while i < 16 {
        palette[i] = 0xff_00_00_00
            | (palette16[i].0 as u32)
            | (palette16[i].1 as u32) << 8
            | (palette16[i].2 as u32) << 16;
        i += 1;
    }

    let mut r = 0;
    let mut g = 0;
    let mut b = 0;

    while b < 6 {
        while g < 6 {
            while r < 6 {
                palette[(16 + r + g * 6 + b * 36) as usize] = 0xff_00_00_00
                    | ((r * 51) as u32)
                    | ((g * 51) as u32) << 8
                    | ((b * 51) as u32) << 16;
                r += 1;
            }
            r = 0;
            g += 1;
        }
        g = 0;
        b += 1;
    }

    palette
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u32);

pub struct HariWindow {
    window_id: WindowId,
    size: Size,
    buffer_ptr: u32,
    buffer_len: usize,
    rgba_buffer: UnsafeCell<Box<[u8]>>,
}

impl HariWindow {
    const WINDOW_BGCOLOR: u8 = 8;
    const WINDOW_ADJUST_X: i32 = 2;
    const WINDOW_ADJUST_TOP: i32 = 22;
    const WINDOW_ADJUST_BOTTOM: i32 = 2;

    /// Creates a new window with the specified `title` and `size`.
    ///
    /// NOTE: The window buffer of Haribote OS includes the window frame and title bar, so it is necessary to adjust the window size considering the height of the host OS's title bar.
    pub fn new(context: &App, title: &JisString, size: Size, buffer_ptr: u32) -> Self {
        let title = title.to_str().unwrap_or("Untitled");
        let width = size.width - (Self::WINDOW_ADJUST_X * 2) as u32;
        let height = size.height - (Self::WINDOW_ADJUST_TOP + Self::WINDOW_ADJUST_BOTTOM) as u32
            + context.title_bar_height;
        let window_id = WindowId(js_open_window(
            width,
            height,
            title.as_ptr(),
            title.len() as u32,
        ));
        let mut rgba_buffer = Vec::with_capacity((size.width * size.height * 4) as usize);
        rgba_buffer.resize((size.width * size.height * 4) as usize, 0);

        let window = Self {
            window_id,
            size,
            buffer_ptr,
            buffer_len: (size.width * size.height) as usize,
            rgba_buffer: UnsafeCell::new(rgba_buffer.into_boxed_slice()),
        };

        window.buffer(context).fill(Self::WINDOW_BGCOLOR);
        window.redraw_rect(
            context,
            Point::ZERO,
            Point::new(size.width as i32, size.height as i32),
        );

        window
    }

    /// Returns window size
    #[inline]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// Returns window's bitmap buffer.
    #[inline]
    fn buffer<'a>(&self, context: &'a App) -> &'a mut [u8] {
        // SAFETY: The buffer is valid as long as self exists, and it is safe to access it simultaneously.
        context
            .emulator
            .data()
            .get_mut(self.buffer_ptr as usize..self.buffer_ptr as usize + self.buffer_len)
            .unwrap()
    }

    /// Returns the window ID
    #[inline]
    pub const fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Moves the window to the specified `origin` point.
    #[inline]
    pub fn move_to(&self, origin: Point) {
        js_move_window(self.window_id.0, origin.x, origin.y);
    }

    /// Clips a point to the window's drawable area. Returns `None` if the point is outside the drawable area.
    #[inline]
    pub fn try_clip_point(&self, point: Point) -> Option<Point> {
        let left = Self::WINDOW_ADJUST_X;
        let top = Self::WINDOW_ADJUST_TOP;
        let right = self.size.width as i32 - Self::WINDOW_ADJUST_X - 1;
        let bottom = self.size.height as i32 - Self::WINDOW_ADJUST_BOTTOM - 1;

        if point.x < left || point.x > right || point.y < top || point.y > bottom {
            None
        } else {
            Some(point)
        }
    }

    /// Clips a rectangle defined by `left_top` and `right_bottom` to the window's drawable area. Returns `None` if the rectangle is completely outside the drawable area.
    pub fn try_clip_rect(&self, left_top: Point, right_bottom: Point) -> Option<(Point, Point)> {
        let min_x = left_top.x.min(right_bottom.x);
        let max_x = left_top.x.max(right_bottom.x);
        let min_y = left_top.y.min(right_bottom.y);
        let max_y = left_top.y.max(right_bottom.y);

        let left = Self::WINDOW_ADJUST_X;
        let top = Self::WINDOW_ADJUST_TOP;
        let right = self.size.width as i32 - Self::WINDOW_ADJUST_X - 1;
        let bottom = self.size.height as i32 - Self::WINDOW_ADJUST_BOTTOM - 1;

        if min_x > right || max_x < left || min_y > bottom || max_y < top {
            None
        } else {
            let clipped_left = min_x.max(left);
            let clipped_right = max_x.min(right);
            let clipped_top = min_y.max(top);
            let clipped_bottom = max_y.min(bottom);
            Some((
                Point::new(clipped_left, clipped_top),
                Point::new(clipped_right, clipped_bottom),
            ))
        }
    }

    /// Redraws a rectangle defined by `left_top` and `right_bottom` on the window.
    pub fn redraw_rect(&self, context: &App, left_top: Point, right_bottom: Point) {
        if let Some((clipped_left_top, clipped_right_bottom)) =
            self.try_clip_rect(left_top, right_bottom)
        {
            let width = (clipped_right_bottom.x - clipped_left_top.x + 1) as usize;
            let height = (clipped_right_bottom.y - clipped_left_top.y + 1) as usize;
            assert!(width <= self.size.width as usize && height <= self.size.height as usize);

            // SAFETY: The buffer is accessed in a single-threaded context, and the slice is valid for the specified width and height.
            let Some(rgba_buffer) =
                unsafe { &mut *self.rgba_buffer.get() }.get_mut(..width * height * 4)
            else {
                return;
            };
            let src_buffer = self.buffer(context);

            {
                // Safety: The buffer is valid for the specified width and height, and the slice is properly aligned for u32 access.
                let rgba_buffer = unsafe {
                    core::slice::from_raw_parts_mut(
                        rgba_buffer.as_mut_ptr() as *mut u32,
                        width * height,
                    )
                };

                let stride = self.size.width as usize;
                let mut base = clipped_left_top.y as usize * stride + clipped_left_top.x as usize;
                for y in 0..height {
                    let draw_base = y * width;
                    let slice = &src_buffer[base..base + width];
                    for (x, &c) in slice.iter().enumerate() {
                        rgba_buffer[draw_base + x] = PALETTE[c as usize];
                    }
                    base += stride;
                }
            }

            js_draw_image(
                self.window_id.0,
                (clipped_left_top.x - Self::WINDOW_ADJUST_X) as u32,
                (clipped_left_top.y - Self::WINDOW_ADJUST_TOP) as u32,
                width as u32,
                height as u32,
                rgba_buffer.as_ptr(),
                rgba_buffer.len() as u32,
            );
        }
    }

    /// Fills a rectangle defined by `left_top` and `right_bottom` with the specified `color`.
    pub fn fill_rect(
        &self,
        context: &App,
        left_top: Point,
        right_bottom: Point,
        color: u8,
        redraw: bool,
    ) {
        if let Some((clipped_left_top, clipped_right_bottom)) =
            self.try_clip_rect(left_top, right_bottom)
        {
            let width = (clipped_right_bottom.x - clipped_left_top.x + 1) as usize;
            let buffer = self.buffer(context);
            let stride = self.size.width as usize;
            let mut base = clipped_left_top.y as usize * stride + clipped_left_top.x as usize;
            for _y in clipped_left_top.y..=clipped_right_bottom.y {
                let slice = &mut buffer[base..base + width];
                slice.fill(color);
                base += stride;
            }
            if redraw {
                self.redraw_rect(context, clipped_left_top, clipped_right_bottom);
            }
        }
    }

    /// Sets the pixel at the specified `point` to the given `color`.
    pub fn set_pixel(&self, context: &App, point: Point, color: u8, redraw: bool) {
        if self.try_clip_point(point).is_none() {
            return;
        }
        let index = (point.y as u32 * self.size.width + point.x as u32) as usize;
        self.buffer(context)[index] = color;
        if redraw {
            self.redraw_rect(context, point, point);
        }
    }

    /// Draws a line from `start` to `end` with the specified `color` using Bresenham's line algorithm.
    pub fn draw_line(&self, context: &App, start: Point, end: Point, color: u8, redraw: bool) {
        let mut x0 = start.x;
        let mut y0 = start.y;
        let x1 = end.x;
        let y1 = end.y;
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        loop {
            self.set_pixel(context, Point::new(x0, y0), color, false);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if e2 < dx {
                err += dx;
                y0 += sy;
            }
        }
        if redraw {
            let left_top = Point::new(start.x.min(end.x), start.y.min(end.y));
            let right_bottom = Point::new(start.x.max(end.x), start.y.max(end.y));
            self.redraw_rect(context, left_top, right_bottom);
        }
    }

    /// Put font data at the specified `origin` point with the given `color`.
    fn put_font_data(&self, context: &App, origin: Point, data: &[u8], color: u8) {
        if origin.x < self.size.width as i32 - 8 && origin.y < self.size.height as i32 - 16 {
            let stride = self.size.width as usize;
            let buffer = self.buffer(context);
            for y in 0..16 {
                let pattern = data[y as usize];
                let cursor = origin.x as usize + (origin.y as usize + y) * stride;
                let line = &mut buffer[cursor..][..8];
                let mut bit = 0x80;
                for pixel in line.iter_mut() {
                    if pattern & bit != 0 {
                        *pixel = color;
                    }
                    bit >>= 1;
                }
            }
        }
    }

    /// Put a single character `ch` at the specified `origin` point with the given `color`.
    pub fn put_font(&self, context: &App, origin: Point, jc: JisChar, color: u8) -> i32 {
        match jc {
            JisChar::ANK(ch) => {
                match ch as u8 {
                    0x21..=0x7f => {
                        let font_stride = 16;
                        let font_offset = (ch as usize - 0x20) * font_stride;
                        let glyph =
                            &fonts::FONT_HANKAKU_DATA[font_offset..font_offset + font_stride];
                        self.put_font_data(context, origin, glyph, color);
                    }
                    0x80..=0xff => {
                        self.fill_rect(
                            context,
                            origin + Point::new(1, 1),
                            origin + Point::new(7, 14),
                            color,
                            false,
                        );
                    }
                    _ => {}
                }
                8
            }
            JisChar::Kanji(kanji) => {
                if origin.x < self.size.width as i32 - 16 && origin.y < self.size.height as i32 - 16
                {
                    let base = 0x1000 + kanji as usize * 32;
                    let font = context.japanese_font();
                    let left = font.get(base..base + 16);
                    let right = font.get(base + 16..base + 32);
                    match (left, right) {
                        (Some(left), Some(right)) => {
                            self.put_font_data(context, origin, left, color);
                            self.put_font_data(context, origin + Point::new(8, 0), right, color);
                        }
                        _ => {
                            // not found, then tofu
                            self.fill_rect(
                                context,
                                origin + Point::new(1, 1),
                                origin + Point::new(14, 14),
                                color,
                                false,
                            );
                        }
                    }
                }
                16
            }
        }
    }

    /// Draws a string `text` at the specified `origin` point with the given `color`, up to `max_len` characters.
    pub fn draw_string(
        &self,
        context: &App,
        origin: Point,
        text: &JisString,
        max_len: usize,
        color: u8,
        redraw: bool,
    ) {
        let mut x = origin.x;
        let y = origin.y;
        for ch in text.chars(context.lang_mode).take(max_len) {
            x += self.put_font(context, Point::new(x, y), ch, color);
        }
        if redraw {
            self.redraw_rect(
                context,
                origin,
                Point::new(origin.x + max_len as i32 * 8, origin.y + 16),
            );
        }
    }
}

impl Drop for HariWindow {
    fn drop(&mut self) {
        js_close_window(self.window_id.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0, y: 0 };

    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Calculates the bottom-right point of a rectangle given its top-left point (`self`) and `size`.
    #[inline]
    pub const fn right_bottom(&self, size: Size) -> Self {
        Self {
            x: self.x + size.width as i32 - 1,
            y: self.y + size.height as i32 - 1,
        }
    }
}

impl Add<Self> for Point {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
