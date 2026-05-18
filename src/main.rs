#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use limine::BaseRevision;
use limine::request::{BootloaderInfoRequest, HhdmRequest, MemmapRequest};

pub mod allocator;
pub mod gdt;
pub mod idt;
pub mod serial;
mod vmm;

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

    let hhdm_offset = HHDM.response().expect("no HHDM from Limine").offset;
    let mmap = MEMORY_MAP.response().expect("no memory map from Limine");
    unsafe { ALLOCATOR.init(mmap.entries(), hhdm_offset) };
    println!(
        "Memory: {} MB free / {} MB total",
        unsafe { ALLOCATOR.free_pages() } * 4 / 1024,
        unsafe { ALLOCATOR.total_pages() } * 4 / 1024,
    );

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

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("KERNEL PANIC: {}", _info);
    loop {}
}
