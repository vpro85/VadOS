use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicU64, Ordering};

/// Начало региона кучи в виртуальном адресном пространстве
pub const HEAP_START: u64 = 0xffff_a000_0000_0000;
/// Размер кучи - 4 MB для начала
pub const HEAP_SIZE: u64 = 4 * 1024 * 1024;

/// Bump allocator - просто двигаем указатель вперед
pub struct BumpAllocator {
    next: AtomicU64, // следующий свободный адрес
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            next: AtomicU64::new(HEAP_START),
        }
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size() as u64;
        let align = layout.align() as u64;

        loop {
            let current = self.next.load(Ordering::Relaxed);

            // Выравниваем адрес
            let aligned = (current + align - 1) & !(align - 1);
            let next = aligned + size;

            if next > HEAP_START + HEAP_SIZE {
                return core::ptr::null_mut(); // out of memory
            }

            // Атомарно двигаем указатель
            if self
                .next
                .compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return aligned as *mut u8;
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator не освобождает память - для начала сойдет и так
    }
}

#[global_allocator]
static ALLOCATOR_HEAP: BumpAllocator = BumpAllocator::new();
