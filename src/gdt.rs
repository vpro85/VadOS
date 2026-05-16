/// Один дескриптор GDT - 8 байт
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Descriptor(u64);

impl Descriptor {
    /// Нулевой дескриптор - обязательно первый в таблице
    pub const fn null() -> Self {
        Self(0)
    }

    /// Дескриптор сегмента кода (64-bit)
    pub const fn kernel_code() -> Self {
        // Биты:
        // 43:      1 = executable (код, не данные)
        // 44:      1 = descriptor type (не системный)
        // 47:      1 = present
        // 53:      1 = 64-bit mode (long mode)
        Self(
            (1 << 43) | // executable
            (1 << 44) | // descriptor type
            (1 << 47) | // present
            (1 << 53)   // 64-bit
        )
    }

    /// Дескриптор сегмента данных
    pub const fn kernel_data() -> Self {
        // Биты:
        // 40:      1 = accessed
        // 41:      1 = writeable
        // 44:      1 = descriptor type
        // 47:      1 = present
        Self(
            (1 << 40) | // accessed (некоторые CPU требуют)
            (1 << 41) | // writeable
            (1 << 44) | // descriptor type
            (1 << 47)   // present
        )
    }
}

/// Селекторы сегментов - индекс * 8
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;

/// Сама таблица - три дескриптора
#[repr(C, align(8))]
pub struct Gdt {
    null: Descriptor,
    code: Descriptor,
    data: Descriptor,
}

/// Структура которую процессор читает через lgdt
#[repr(C, packed)]
pub struct GetRegister {
    limit: u16,     // размер таблицы - 1
    base: u64,      // виртуальный адрес таблицы
}

impl Gdt {
    pub const fn new() -> Self {
        Self {
            null: Descriptor::null(),
            code: Descriptor::kernel_code(),
            data: Descriptor::kernel_data(),
        }
    }

    pub fn descriptor(&self) -> GetRegister {
        GetRegister {
            limit: (core::mem::size_of::<Gdt>() - 1) as u16,
            base: self as *const _ as u64,
        }
    }
}

/// Загружает GDT в процессор и обновляет сегментные регистры
pub fn load(gdt: &'static Gdt) {
    // Безопасно: load() вызывается ровно один раз до запуска
    // любых других потоков, гонок быть не может
    static mut GDTR: GetRegister = GetRegister {limit: 0, base: 0 };

    unsafe {
        GDTR = gdt.descriptor();
        core::arch::asm!("lgdt [{}]", in(reg) &raw const GDTR);

        core::arch::asm!(
            "mov ss, {zero:x}",
            "mov ds, {zero:x}",
            "mov es, {zero:x}",
            "mov fs, {zero:x}",
            "mov gs, {zero:x}",
            zero = in(reg) 0u16,
        );

        core::arch::asm!(
            "sub rsp, 16",
            "mov qword ptr [rsp + 8], {code}",
            "lea rax, [rip + 2f]",
            "mov qword ptr [rsp], rax",
            "rex64 retf",
            "2:",
            code = in(reg) KERNEL_CODE_SELECTOR as u64,
            out("rax") _,
        );
    }
}
