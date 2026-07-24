//! Simple memory allocator

pub struct SimpleAllocator {
    start: usize,
    end: usize,
}

impl SimpleAllocator {
    pub fn new() -> Self {
        SimpleAllocator { start: 0, end: 0 }
    }

    pub fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
    }

    pub fn alloc(&mut self, size: usize) -> Option<usize> {
        let size = (size + 15) & !15;
        if self.start + size > self.end {
            None
        } else {
            let addr = self.start;
            self.start += size;
            Some(addr)
        }
    }
}
