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

    /// Инициализация аллокатора по memory map от Limine
    pub unsafe fn init(&mut self, entries: &[&limine::memmap::Entry]) {
        unsafe {
            // Найдём максимальный физический адрес
            let mut max_addr: u64 = 0;
            for entry in entries {
                let end = entry.base + entry.length;
                if end > max_addr {
                    max_addr = end;
                }
            }

            // Считаем сколько страниц и байт нужно для bitmap
            self.total_pages = (max_addr / PAGE_SIZE) as usize;
            self.bitmap_size = (self.total_pages + 7) / 8;

            // Ищем первый Usable регион достаточного размера для bitmap
            for entry in entries.iter() {
                if entry.type_ == limine::memmap::MEMMAP_USABLE
                    && entry.length >= self.bitmap_size as u64 {
                    self.bitmap = entry.base as *mut u8;
                    break;
                }
            }

            assert!(!self.bitmap.is_null(), "no space for bitmap");

            // Помечаем всё как занятое (все биты = 1)
            core::ptr::write_bytes(self.bitmap, 0xFF, self.bitmap_size);

            // Освобождаем только Usable регионы
            for entry in entries {
                if entry.type_ == limine::memmap::MEMMAP_USABLE {
                    let start_page = (entry.base / PAGE_SIZE) as usize;
                    let end_page = ((entry.base + entry.length) / PAGE_SIZE) as usize;
                    for page in start_page..end_page {
                        self.set_bit(page);
                        self.free_pages += 1;
                    }
                }
            }

            // Помечаем страницы самого bitmap как занятые
            let bitmap_start = self.bitmap as u64;
            let bitmap_end = bitmap_start + self.bitmap_size as u64;
            let start_page = (bitmap_start / PAGE_SIZE) as usize;
            let end_page = ((bitmap_end + PAGE_SIZE - 1) / PAGE_SIZE) as usize;
            for page in start_page..end_page {
                if !self.test_bit(page) {
                    self.set_bit(page);
                    self.free_pages -= 1;
                }
            }
        }
    }
}
