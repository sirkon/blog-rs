
#[inline(always)]
pub fn fast_crc32c(crc: u32, data: &[u8]) -> u32 {
    #[cfg(target_arch = "aarch64")]
    {
        // Проверка в рантайме, но на M4 она почти бесплатна (один флаг в памяти)
        // Если ты собираешь с target-feature=+crc, компилятор может это еще сильнее оптимизировать
        if std::arch::is_aarch64_feature_detected!("crc") {
            return unsafe { crc32c_aarch64_hw(crc, data) };
        }
    }

    // Фоллбэк для x86_64 или старых ARM (используем твой текущий крейт)
    crc32c::crc32c_append(crc, data)
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn crc32c_aarch64_hw(mut crc: u32, data: &[u8]) -> u32 { 
    use core::arch::aarch64::{__crc32cb, __crc32cd, __crc32ch, __crc32cw};
    
    let mut ptr = data.as_ptr();
    let mut len = data.len();

    // Инвертируем вход, так как инструкции работают с прямым значением,
    // а стандарт требует инверсии (как в начале crc32c)
    crc = !crc;

    // 1. Head: Выравниваем указатель до 8 байт (u64)
    // Это убирает "софтовый чит" из оригинальной либы
    while len > 0 && (ptr as usize & 7) != 0 {
        crc = __crc32cb(crc, *ptr);
        ptr = ptr.add(1);
        len -= 1;
    }

    // 2. Body: Шпарим по 8 байт за раз. Это основной буст для M4.
    // Мы не делим на "параллельные блоки по 3", так как на мелких записях
    // оверхед на склейку таблиц (как в либе) съедает весь профит.
    while len >= 8 {
        crc = __crc32cd(crc, *(ptr as *const u64));
        ptr = ptr.add(8);
        len -= 8;
    }

    // 3. Tail: Добиваем остатки (4, 2, 1 байт)
    if len >= 4 {
        crc = __crc32cw(crc, *(ptr as *const u32));
        ptr = ptr.add(4);
        len -= 4;
    }
    if len >= 2 {
        crc = __crc32ch(crc, *(ptr as *const u16));
        ptr = ptr.add(2);
        len -= 2;
    }
    if len > 0 {
        crc = __crc32cb(crc, *ptr);
    }

    // Финальная инверсия
    !crc
}
