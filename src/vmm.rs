/// Флаги записи в таблице страниц
pub mod flags {
    pub const PRESENT: u64 = 1 << 0; // страница существует
    pub const WRITABLE: u64 = 1 << 1; // можно писать
    pub const USER: u64 = 1 << 2; // доступна из ring 3
    pub const NO_EXECUTE: u64 = 1 << 63; // нельзя исполнять
}

/// Одна запись в таблице страниц (PML4E, PDPTE, TDE, PTE)
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageEntry(u64);

impl PageEntry {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn is_present(&self) -> bool {
        self.0 & flags::PRESENT != 0
    }

    /// Физический адрес следующей таблицы или страницы
    pub fn phys_addr(&self) -> u64 {
        self.0 & 0x000FFFFF_FFFFF000
    }

    /// Создать запись указывающую на физический адрес с флагами
    pub fn new(phys: u64, flags: u64) -> Self {
        Self((phys & 0x000FFFFF_FFFFF000) | flags)
    }
}

/// Таблица страниц - 512 записей
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageEntry; 512],
}

impl PageTable {
    pub const fn empty() -> Self {
        Self {
            entries: [PageEntry::empty(); 512],
        }
    }
}

/// Извлекает индекс в таблице для каждого уровня
pub fn pml4_index(virt: u64) -> usize {
    ((virt >> 39) & 0x1FF) as usize
}
pub fn pdpt_index(virt: u64) -> usize {
    ((virt >> 30) & 0x1FF) as usize
}
pub fn pd_index(virt: u64) -> usize {
    ((virt >> 21) & 0x1FF) as usize
}
pub fn pt_index(virt: u64) -> usize {
    ((virt >> 12) & 0x1FF) as usize
}

/// Получить указатель на таблицу страниц по физическому адресу
/// (через HHDM смещение)
unsafe fn phys_to_table(phys: u64, hhdm: u64) -> *mut PageTable {
    (phys + hhdm) as *mut PageTable
}

/// Получить или создать следующий уровень таблицы
/// Если запись пустая - выделяем новую страницу через frame allocator
unsafe fn get_or_create(entry: &mut PageEntry, hhdm: u64) -> *mut PageTable {
    if entry.is_present() {
        unsafe { phys_to_table(entry.phys_addr(), hhdm) }
    } else {
        // Выделяем новую физическую страницу
        let phys = unsafe { crate::ALLOCATOR.alloc_frame().expect("vmm: out of memory") };

        // Обнуляем новую таблицу
        let table = unsafe { phys_to_table(phys, hhdm) };
        unsafe { core::ptr::write_bytes(table, 0, 1) };

        // Записываем в родительскую запись
        *entry = PageEntry::new(phys, flags::PRESENT | flags::WRITABLE);

        table
    }
}

/// Мапит одну страницу: virt -> phys с заданными флагами
/// hhdm - смещение из Limine HhdmRequest
pub unsafe fn map_page(virt: u64, phys: u64, page_flags: u64, hhdm: u64) {
    unsafe {
        // Читаем CR3 - физический адрес PML4
        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
        let pml4 = phys_to_table(cr3 & 0x000FFFFF_FFFFF000, hhdm);

        let pdpt = get_or_create(&mut (*pml4).entries[pml4_index(virt)], hhdm);
        let pd = get_or_create(&mut (*pdpt).entries[pdpt_index(virt)], hhdm);
        let pt = get_or_create(&mut (*pd).entries[pd_index(virt)], hhdm);

        // Записываем финальный маппинг
        (*pt).entries[pt_index(virt)] = PageEntry::new(phys, page_flags);
    }
}
