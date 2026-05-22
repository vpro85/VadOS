#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use limine::BaseRevision;
use limine::request::{BootloaderInfoRequest, HhdmRequest, MemmapRequest};

pub mod allocator;
pub mod gdt;
pub mod heap;
pub mod idt;
pub mod pic;
pub mod pit;
pub mod serial;
pub mod vmm;

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

#[used]
static HHDM: HhdmRequest = HhdmRequest::new();

static GDT: gdt::Gdt = gdt::Gdt::new();

static IDT: idt::IdtCell = idt::IdtCell::new();

extern crate alloc;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("VadOS kernel starting...");

    gdt::load(&GDT);
    println!("GDT loaded");

    unsafe {
        IDT.set_handler(0, idt::handler_divide_error as *const () as u64);
        IDT.set_handler(13, idt::handler_gpf as *const () as u64);
        IDT.set_handler(14, idt::handler_page_fault as *const () as u64);
        IDT.set_handler(255, idt::handler_unhandled as *const () as u64);
        IDT.load();
    }
    println!("IDT loaded");

    unsafe {
        IDT.set_handler(0x20, idt::handler_timer as *const () as u64);
    }

    unsafe { pic::init() };
    unsafe { pit::init() };
    println!("PIC and PIT initialized");

    // Включаем прерывания
    unsafe { core::arch::asm!("sti") };
    println!("Interrupts enabled");

    let hhdm_offset = HHDM.response().expect("no HHDM from Limine").offset;
    let mmap = MEMORY_MAP.response().expect("no memory map from Limine");
    unsafe { ALLOCATOR.init(mmap.entries(), hhdm_offset) };
    println!(
        "Memory: {} MB free / {} MB total",
        unsafe { ALLOCATOR.free_pages() } * 4 / 1024,
        unsafe { ALLOCATOR.total_pages() } * 4 / 1024,
    );

    // Мапим страницы под кучу
    let heap_pages = heap::HEAP_SIZE / 4096;
    for i in 0..heap_pages {
        let phys = unsafe { ALLOCATOR.alloc_frame() }.expect("heap: out of memory");
        let virt = heap::HEAP_START + i * 4096;
        unsafe {
            vmm::map_page(
                virt,
                phys,
                vmm::flags::PRESENT | vmm::flags::WRITABLE,
                hhdm_offset,
            );
        }
    }
    println!("Heap initialized: {} MB", heap::HEAP_SIZE / 1024 / 1024);

    unsafe { heap::ALLOCATOR_HEAP.init() };

    // Тест кучи
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    let b = Box::new(42u64);
    println!("Box: {}", b);

    let mut v: Vec<u64> = Vec::new();
    for i in 0..5 {
        v.push(i * i);
    }
    println!("Vec: {:?}", v);

    // Тест виртуальной памяти:
    // выделим физическую страницу и замапим её по произвольному виртуальному адресу
    let phys = unsafe { ALLOCATOR.alloc_frame() }.expect("alloc failed");
    let virt = 0xffff_9000_0000_0000u64; // произвольный адрес в верхней половине

    unsafe {
        vmm::map_page(
            virt,
            phys,
            vmm::flags::PRESENT | vmm::flags::WRITABLE,
            hhdm_offset,
        )
    }

    // Пишем в вирткальный адрес и читаем обратно
    unsafe {
        let ptr = virt as *mut u64;
        *ptr = 0xDEADBEEFu64;
        let val = *ptr;
        println!("vmm test: wrote 0xDEADBEEF, read 0x{:x}", val);
    }

    let a = unsafe { ALLOCATOR.alloc_frame() }.expect("alloc failed");
    let b = unsafe { ALLOCATOR.alloc_frame() }.expect("alloc failed");
    let c = unsafe { ALLOCATOR.alloc_frame() }.expect("alloc failed");
    println!("Allocated: 0x{:x}, 0x{:x}, 0x{:x}", a, b, c);

    unsafe { ALLOCATOR.free_frame(b) };
    let d = unsafe { ALLOCATOR.alloc_frame() }.expect("alloc failed");
    println!("Freed 0x{:x}, reallocated as 0x{:x}", b, d);

    let mut last_sec = 0u64;
    loop {
        let ticks = idt::TICKS.load(core::sync::atomic::Ordering::Relaxed);
        let sec = ticks / pit::PIT_FREQ as u64;

        if sec != last_sec {
            println!("Uptime: {} sec (ticks: {})", sec, ticks);
            last_sec = sec;
        }

        unsafe { core::arch::asm!("hlt") };
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("KERNEL PANIC: {}", _info);
    loop {}
}
