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

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
