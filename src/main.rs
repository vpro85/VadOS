#![no_std]
#![no_main]

use core::panic::PanicInfo;
use limine::request::{BootloaderInfoRequest, MemmapRequest};
use limine::BaseRevision;

pub mod allocator;
pub mod serial;

// Говорим Limine, что поддерживаем протокол версии 3
#[used]
static BASE_REVISION: BaseRevision = BaseRevision::new();

// Запрашиваем информацию о загрузчике
#[used]
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();

#[used]
static MEMORY_MAP: MemmapRequest = MemmapRequest::new();

#[used]
static ALLOCATOR: allocator::AllocatorCell = allocator::AllocatorCell::new();

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("VadOS kernel starting...");
    let mmap = MEMORY_MAP
        .response()
        .expect("no memory map from Limine");

    unsafe {
        ALLOCATOR.init(mmap.entries());
    }

    println!(
        "Memory: {} MB free / {} MB total",
        unsafe { ALLOCATOR.free_pages() } * 4 /1024,
        unsafe { ALLOCATOR.total_pages() } * 4 /1024,
    );

    // Тест: выделим три страницы и освободим одну
    let a = unsafe { ALLOCATOR.alloc_frame() }.expect("alloc failed");
    let b = unsafe { ALLOCATOR.alloc_frame() }.expect("alloc failed");
    let c = unsafe { ALLOCATOR.alloc_frame() }.expect("alloc failed");

    println!("Allocated: 0x{:x}, 0x{:x}, 0x{:x}", a, b, c);

    unsafe { ALLOCATOR.free_frame(b) };
    println!("Freed:     0x{:x}", b);

    let d = unsafe { ALLOCATOR.alloc_frame() }.expect("alloc failed");
    println!("Allocated: 0x{:x}", d);

    println!("Free pages after test: {}", unsafe { ALLOCATOR.free_pages() });

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("KERNEL PANIC: {}", _info);
    loop {}
}
