//! File manager

use alloc::collections::BTreeMap;

use crate::haribote::Handle;
use crate::*;

pub struct FileManager {
    files: BTreeMap<Handle, File>,
    next_handle: u32,
}

pub struct File {
    #[allow(dead_code)]
    name: String,
    data: Vec<u8>,
    pos: isize,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            next_handle: 1,
        }
    }

    /// Open a file for reading.
    pub fn open_file(&mut self, filename: &str) -> Option<Handle> {
        let data = rust_read_file(filename)?;
        let file = File {
            name: filename.to_string(),
            data,
            pos: 0,
        };
        let handle = Handle(self.next_handle);
        self.next_handle += 1;
        self.files.insert(handle, file);
        Some(handle)
    }

    /// Open a file for writing.
    pub fn open_file_for_write(&mut self, filename: &str) -> Option<Handle> {
        let _ = filename;
        todo!()
    }

    #[inline]
    pub fn get(&mut self, handle: Handle) -> Option<&mut File> {
        self.files.get_mut(&handle)
    }

    #[inline]
    pub fn close_file(&mut self, handle: Handle) -> Option<()> {
        self.files.remove(&handle).map(|_| ())
    }
}

impl File {
    pub fn seek(&mut self, offset: isize, whence: Whence) -> Option<isize> {
        let new_fp = match whence {
            Whence::SeekSet => offset,
            Whence::SeekCur => self.pos + offset,
            Whence::SeekEnd => self.data.len() as isize + offset,
        };
        if new_fp < 0 {
            self.pos = 0;
            None
        } else if new_fp > self.data.len() as isize {
            self.pos = self.data.len() as isize;
            None
        } else {
            self.pos = new_fp;
            Some(self.pos)
        }
    }

    pub fn get_file_size(&self, whence: Whence) -> isize {
        match whence {
            Whence::SeekSet => self.data.len() as isize,
            Whence::SeekCur => self.pos,
            Whence::SeekEnd => self.pos - self.data.len() as isize,
        }
    }

    pub fn read<'a>(&'a mut self, max_size: usize) -> &'a [u8] {
        let max_size = max_size.min(self.data.len() - self.pos as usize);
        let data = &self.data[self.pos as usize..self.pos as usize + max_size];
        self.pos += max_size as isize;
        data
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Whence {
    #[default]
    SeekSet,
    SeekCur,
    SeekEnd,
}

impl Whence {
    #[inline]
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Whence::SeekSet),
            1 => Some(Whence::SeekCur),
            2 => Some(Whence::SeekEnd),
            _ => None,
        }
    }
}
