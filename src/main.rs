#![no_std]
#![no_main]

use core::panic::PanicInfo;
use limine::request::{BootloaderInfoRequest, MemmapRequest};
use limine::memmap;
use limine::BaseRevision;

pub mod serial;

// Говорим Limine, что поддерживаем протокол версии 3
#[used]
static BASE_REVISION: BaseRevision = BaseRevision::new();

// Запрашиваем информацию о загрузчике
#[used]
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();

#[used]
static MEMORY_MAP: MemmapRequest = MemmapRequest::new();

fn memmap_type_name(type_: u64) -> &'static str {
    match type_ {
        memmap::MEMMAP_USABLE => "Usable",
        memmap::MEMMAP_RESERVED => "Reserved",
        memmap::MEMMAP_ACPI_RECLAIMABLE => "ACPI reclaimable",
        memmap::MEMMAP_ACPI_NVS => "ACPI NVS",
        memmap::MEMMAP_BAD_MEMORY => "Bad memory",
        memmap::MEMMAP_BOOTLOADER_RECLAIMABLE => "Bootloader reclaimable",
        memmap::MEMMAP_EXECUTABLE_AND_MODULES => "Kernel/modules",
        memmap::MEMMAP_FRAMEBUFFER => "Framebuffer",
        _ => "Unknown",
    }
}
#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("Vad OS kernel starting...");
    let mmap = MEMORY_MAP
        .response()
        .expect("no memory map from limine");

    let entries = mmap.entries();
    println!("Memory map ({} entries):", entries.len());

    let mut usable_bytes: u64 = 0;
    for entry in entries {
        println!(
            " 0x{:012x} - 0x{:012x} {}",
            entry.base,
            entry.base + entry.length,
            memmap_type_name(entry.type_)
        );
        if entry.type_ == memmap::MEMMAP_USABLE {
            usable_bytes += entry.length;
        }
    }

    println!("Total usable memory: {} MB", usable_bytes / 1024 / 1024);

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("KERNEL PANIC: {}", _info);
    loop {}
}
