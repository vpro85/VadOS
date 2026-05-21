/// Порты PIT
const PIT_CHANNEL0: u16 = 0x40; // счетчик канала 0
const PIT_COMMAND: u16 = 0x43; // командный регистр

/// Базовая частота PIT в Hz
const PIT_BASE_FREQ: u32 = 1_193_182;

/// Желаемая частота прерываний
pub const PIT_FREQ: u32 = 100; // 100 Hz = прерывание каждые 10мс

/// Настройка PIT на заданную частоту
pub unsafe fn init() {
    let divisor = PIT_BASE_FREQ / PIT_FREQ;

    unsafe {
        //  Канал 0, доступ lo/hi, режим 3 (square wave), двоичный
        crate::pic::outb(PIT_COMMAND, 0x36);

        // Записываем делитель - сначала младший байт, потом старший
        crate::pic::outb(PIT_CHANNEL0, (divisor & 0xFF) as u8);
        crate::pic::outb(PIT_CHANNEL0, (divisor >> 8 & 0xFF) as u8);
    }
}
