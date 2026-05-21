/// Порты мастер PIC
const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;

/// Порты слейв PIC
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

/// Команды PIC
const PIC_EOI: u8 = 0x20; // End of Interrupt
const ICW1_INIT: u8 = 0x11; // инициализация + ICW4 needed
const ICW4_8086: u8 = 0x01; // режим 8086

/// Векторы после ремаппнга
pub const PIC1_OFFSET: u8 = 0x20; // IRQ0-7 -> векторы 0x20-0x27
pub const PIC2_OFFSET: u8 = 0x28; // IRQ8-15 -> векторы 0x28-0x2F

/// Записать байт в порт
unsafe fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        );
    }
}

/// Прочитать байт из порта
unsafe fn inb(port: u16) -> u8 {
    unsafe {
        let value: u8;
        core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        );
        value
    }
}

/// Небольшая задержка через запись в порт 0x80 (POST порт)
unsafe fn io_wait() {
    unsafe { outb(0x80, 0) };
}

/// Инициализация и ремапинг PIC
pub unsafe fn init() {
    unsafe {
        // Начинаем инициализацию (ICW1)
        outb(PIC1_COMMAND, ICW1_INIT);
        io_wait();
        outb(PIC2_COMMAND, ICW1_INIT);
        io_wait();

        // Устанавливаем векторные смещения (ICW2)
        outb(PIC1_DATA, PIC1_OFFSET);
        io_wait();
        outb(PIC2_DATA, PIC2_OFFSET);
        io_wait();

        // Настраиваем каскад (ICW3)
        outb(PIC1_DATA, 0x04); // мастер: слейв на IRQ2
        io_wait();
        outb(PIC2_DATA, 0x02); // слейв: каскад через IRQ2
        io_wait();

        // Режим 8086 (ICW4)
        outb(PIC1_DATA, ICW4_8086);
        io_wait();
        outb(PIC2_DATA, ICW4_8086);
        io_wait();

        // Маскируем все прерывания кроме IRQ0 (таймер) и IRQ2 (каскад)
        // Маска: 1 = замаскировано, 0 = разрешено
        outb(PIC1_DATA, 0b11111000); // Разрешаем IRQ0, IRQ1, IRQ2
        outb(PIC2_DATA, 0b11111111); // Маскируем всё на слейве
    }
}

/// Сигнал конца прерывания - обязательно вызвать в конце каждого обработчика
pub unsafe fn end_of_interrupt(irq: u8) {
    unsafe {
        if irq >= 8 {
            outb(PIC2_COMMAND, PIC_EOI);
        }
        outb(PIC1_DATA, PIC_EOI);
    }
}
