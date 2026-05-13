#![no_std]
#![no_main]

use core::panic::PanicInfo;
use limine::request::BootloaderInfoRequest;
use limine::BaseRevision;

// Говорим Limine, что поддерживаем протокол версии 3
#[used]
static BASE_REVISION: BaseRevision = BaseRevision::new();

// Запрашиваем информацию о загрузчике
#[used]
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();

fn serial_write_byte(byte: u8) {
    unsafe {
        core::arch::asm!(
            "out dx,al",
            in("dx") 0x3F8u16,
            in("al") byte,
        );
    }
}
fn serial_write(s: &str) {
    for byte in s.bytes() {
        serial_write_byte(byte);
    }
}

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    serial_write("Hello from Vad OS kernel!\n");
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_write("KERNEL PANIC!\n");
    loop {}
}
