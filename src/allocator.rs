const PAGE_SIZE: u64 = 4096;

pub struct BitmapAllocator {
    bitmap: *mut u8,
    bitmap_size: usize,
    total_pages: usize,
    free_pages: usize,
}

impl BitmapAllocator {
    pub const fn new() -> Self {
        Self {
            bitmap: core::ptr::null_mut(),
            bitmap_size: 0,
            total_pages: 0,
            free_pages: 0,
        }
    }

    fn set_bit(&mut self, page: usize) {
        unsafe {
            let byte = self.bitmap.add(page / 8);
            *byte |= 1 << (page % 8);
        }
    }

    fn clear_bit(&mut self, page: usize) {
        unsafe {
            let byte = self.bitmap.add(page / 8);
            *byte &= !(1 << (page % 8));
        }
    }

    fn test_bit(&self, page: usize) -> bool {
        unsafe {
            let byte = self.bitmap.add(page / 8);
            (*byte >> (page % 8)) & 1 == 1
        }
    }

    pub unsafe fn init(&mut self, entries: &[&limine::memmap::Entry], hhdm_offset: u64) {
        unsafe {
            let mut max_addr: u64 = 0;
            for entry in entries {
                if entry.type_ == limine::memmap::MEMMAP_USABLE {
                    let end = entry.base + entry.length;
                    if end > max_addr {
                        max_addr = end;
                    }
                }
            }

            self.total_pages = (max_addr / PAGE_SIZE) as usize;
            self.bitmap_size = (self.total_pages + 7) / 8;

            // Отладка: выводим размер bitmap и первые Usable регионы
            crate::println!("bitmap_size = {} bytes", self.bitmap_size);
            for entry in entries.iter() {
                if entry.type_ == limine::memmap::MEMMAP_USABLE {
                    crate::println!(
                        "  usable: 0x{:x} len=0x{:x} ({}KB)",
                        entry.base,
                        entry.length,
                        entry.length / 1024
                    );
                }
            }

            // Ищем первый Usable регион для bitmap
            let mut bitmap_phys: Option<u64> = None;
            for entry in entries.iter() {
                if entry.type_ == limine::memmap::MEMMAP_USABLE
                    && entry.length >= self.bitmap_size as u64
                {
                    bitmap_phys = Some(entry.base);
                    break;
                }
            }

            let bitmap_phys = bitmap_phys.expect("no space for bitmap");

            // Виртуальный адрес = физический + HHDM смещение
            self.bitmap = (bitmap_phys + hhdm_offset) as *mut u8;

            // Помечаем всё как занятое
            core::ptr::write_bytes(self.bitmap, 0xFF, self.bitmap_size);

            // Освобождаем Usable регионы
            for entry in entries {
                if entry.type_ == limine::memmap::MEMMAP_USABLE {
                    let start_page = (entry.base / PAGE_SIZE) as usize;
                    let end_page = ((entry.base + entry.length) / PAGE_SIZE) as usize;
                    for page in start_page..end_page {
                        self.clear_bit(page);
                        self.free_pages += 1;
                    }
                }
            }

            // Помечаем страницы самого bitmap как занятые
            let bitmap_end = bitmap_phys + self.bitmap_size as u64;
            let start_page = (bitmap_phys / PAGE_SIZE) as usize;
            let end_page = ((bitmap_end + PAGE_SIZE - 1) / PAGE_SIZE) as usize;
            for page in start_page..end_page {
                if !self.test_bit(page) {
                    self.set_bit(page);
                    self.free_pages -= 1;
                }
            }
        }
    }

    pub fn alloc_frame(&mut self) -> Option<u64> {
        for page in 0..self.total_pages {
            if !self.test_bit(page) {
                self.set_bit(page);
                self.free_pages -= 1;
                return Some(page as u64 * PAGE_SIZE);
            }
        }
        None
    }

    pub fn free_frame(&mut self, addr: u64) {
        let page = (addr / PAGE_SIZE) as usize;
        assert!(page < self.total_pages, "free_frame: address out of range");
        assert!(self.test_bit(page), "free_frame: page already free");
        self.clear_bit(page);
        self.free_pages += 1;
    }

    pub fn free_pages(&self) -> usize {
        self.free_pages
    }

    pub fn total_pages(&self) -> usize {
        self.total_pages
    }
}

pub struct AllocatorCell {
    inner: core::cell::UnsafeCell<BitmapAllocator>,
}

unsafe impl Sync for AllocatorCell {}

impl AllocatorCell {
    pub const fn new() -> Self {
        Self {
            inner: core::cell::UnsafeCell::new(BitmapAllocator::new()),
        }
    }

    pub unsafe fn init(&self, entries: &[&limine::memmap::Entry], hhdm_offset: u64) {
        unsafe { (*self.inner.get()).init(entries, hhdm_offset) }
    }

    pub unsafe fn alloc_frame(&self) -> Option<u64> {
        unsafe { (*self.inner.get()).alloc_frame() }
    }

    pub unsafe fn free_frame(&self, addr: u64) {
        unsafe { (*self.inner.get()).free_frame(addr) }
    }

    pub unsafe fn free_pages(&self) -> usize {
        unsafe { (*self.inner.get()).free_pages() }
    }

    pub unsafe fn total_pages(&self) -> usize {
        unsafe { (*self.inner.get()).total_pages() }
    }
}
