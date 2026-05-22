use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

/// Начало региона кучи в виртуальном адресном пространстве
pub const HEAP_START: u64 = 0xffff_a000_0000_0000;
/// Размер кучи - 4 MB для начала
pub const HEAP_SIZE: u64 = 4 * 1024 * 1024;

/// Минимальный размер блока - чтобы после split оставалось место для заголовка
const MIN_BLOCK_SIZE: usize = core::mem::size_of::<FreeBlock>();

/// Заголовок свободного блока в списке
struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

/// Linked list allocator
pub struct LinkedListAllocator {
    head: core::cell::UnsafeCell<*mut FreeBlock>,
}

unsafe impl Sync for LinkedListAllocator {}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self {
            head: core::cell::UnsafeCell::new(ptr::null_mut()),
        }
    }

    /// Инициализация - добавляем весь heap как один большой свободный блок
    pub unsafe fn init(&self) {
        unsafe {
            let block = HEAP_START as *mut FreeBlock;
            (*block).size = HEAP_SIZE as usize - core::mem::size_of::<FreeBlock>();
            (*block).next = ptr::null_mut();
            *self.head.get() = block;
        }
    }

    /// Выравнивает адрес вверх до нужного выравнивания
    fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }
}

unsafe impl GlobalAlloc for LinkedListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(MIN_BLOCK_SIZE);
        let align = layout.align();

        unsafe {
            let head = self.head.get();
            let mut current = head as *mut *mut FreeBlock;

            // Идем по списку свободных блоков
            while !(*current).is_null() {
                let block = *current;
                let block_addr = block as usize;

                // Считаем выровненный адрес начала данных
                let data_start = Self::align_up(
                    block_addr + core::mem::size_of::<FreeBlock>(),
                    align,
                );
                let data_end = data_start + size;
                let block_end = block_addr + core::mem::size_of::<FreeBlock>() + (*block).size;

                if data_end <= block_end {
                    // Блок подходит - проверяем можно ли разбить
                    let remaining = block_end - data_end;
                    if remaining >= core::mem::size_of::<FreeBlock>() + MIN_BLOCK_SIZE {
                        // Создаем новый свободный блок из остатка
                        let new_block = data_end as *mut FreeBlock;
                        (*new_block).size = remaining - core::mem::size_of::<FreeBlock>();
                        (*new_block).next = (*block).next;
                        *current = new_block;
                    } else {
                        // Остаток слишком мал - отдаем весь блок
                        *current = (*block).next;
                    }
                    return data_start as *mut u8;
                }

                current = &mut (*block).next as *mut *mut FreeBlock;
            }

            ptr::null_mut() // out of memory
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(MIN_BLOCK_SIZE);
        unsafe {
            // Создаем новый свободный блок
            let block = ptr as *mut FreeBlock;
            (*block).size = size;

            // Вставляем в начало списка
            let head = self.head.get();
            (*block).next = *head;
            *head = block;
        }
    }
}

#[global_allocator]
pub static ALLOCATOR_HEAP: LinkedListAllocator = LinkedListAllocator::new();
