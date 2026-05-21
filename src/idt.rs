/// Дескриптор прерывания 16 бит
#[derive(Clone, Copy)]
#[repr(C)]
pub struct IdtEntry {
    offset_low: u16,  // биты 15:0 адреса обработчика
    selector: u16,    // селектор сегмента кода
    ist: u8,          // interrupt stack table (0 = не используем)
    flags: u8,        // type + DPL + present
    offset_mid: u16,  // биты 31:16 адреса обработчика
    offset_high: u32, // биты 63:32 адреса обработчика
    _reserved: u32,   // всегда 0
}

impl IdtEntry {
    pub const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            flags: 0,
            offset_mid: 0,
            offset_high: 0,
            _reserved: 0,
        }
    }

    pub fn new(handler: u64) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector: crate::gdt::KERNEL_CODE_SELECTOR,
            ist: 0,
            flags: 0x8E, // present + interrupt gate + DPL=0
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            _reserved: 0,
        }
    }
}

/// Регистр IDTR - аналог GdtRegister
#[repr(C, packed)]
pub struct IdtRegister {
    limit: u16,
    base: u64,
}

/// Сама таблица - 256 дескрипторов
#[repr(C, align(16))]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::missing(); 256],
        }
    }

    pub fn set_handler(&mut self, vector: u8, handler: u64) {
        self.entries[vector as usize] = IdtEntry::new(handler);
    }

    pub fn load(&'static self) {
        let idtr = IdtRegister {
            limit: (core::mem::size_of::<Idt>() - 1) as u16,
            base: self as *const _ as u64,
        };
        unsafe {
            core::arch::asm!("lidt [{}]", in(reg) &idtr);
        }
    }
}

/// Контекст, который процессор кладет на стек при исключении
#[repr(C)]
pub struct InterruptFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/// Обработчик исключения без error code
pub extern "x86-interrupt" fn handler_divide_error(frame: InterruptFrame) {
    panic!("Division by zero at 0x{:x}", frame.rip);
}

/// Обработчик general protection fault (с error code)
pub extern "x86-interrupt" fn handler_gpf(frame: InterruptFrame, error: u64) {
    panic!(
        "General protection fault at 0x{:x}, error=0x{:x}",
        frame.rip, error
    );
}

/// Обработчик page fault (с error code)
pub extern "x86-interrupt" fn handler_page_fault(frame: InterruptFrame, error: u64) {
    panic!("Page fault at 0x{:x}, error=0x{:x}", frame.rip, error);
}

/// Заглушка для необработанных прерываний
pub extern "x86-interrupt" fn handler_unhandled(frame: InterruptFrame) {
    panic!("Unhandled interrupt at 0x{:x}", frame.rip);
}

pub struct IdtCell {
    inner: core::cell::UnsafeCell<Idt>,
}

unsafe impl Sync for IdtCell {}

impl IdtCell {
    pub const fn new() -> Self {
        Self {
            inner: core::cell::UnsafeCell::new(Idt::new()),
        }
    }

    pub unsafe fn set_handler(&self, vector: u8, handler: u64) {
        unsafe { (*self.inner.get()).set_handler(vector, handler) }
    }

    pub fn load(&'static self) {
        unsafe { (*self.inner.get()).load() }
    }
}

use core::sync::atomic::{AtomicU64, Ordering};

/// Счетчик тиков таймера
pub static TICKS: AtomicU64 = AtomicU64::new(0);

/// Обработчик IRQ0 - таймер
pub extern "x86-interrupt" fn handler_timer(frame: InterruptFrame) {
    let _ = frame;
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe { crate::pic::end_of_interrupt(0) };
}
