#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use jiff::Timestamp;

#[inline(always)]
pub(crate) fn render_time(rendbuf: &mut itoa::Buffer, dst: &mut Vec<u8>, nanos: i64) {
    let secs = nanos / 1_000_000_000;
    let nsecs = (nanos % 1_000_000_000) as i32;

    // Создаём Timestamp из секунд и наносекунд
    let ts = match Timestamp::new(secs, nsecs) {
        Ok(ts) => ts,
        _ => {
            dst.extend_from_slice(b"????-??-?? ??:??:??.???");
            return;
        }
    };

    // Конвертируем в локальный часовой пояс
    let zoned = ts.to_zoned(jiff::tz::TimeZone::system());

    // Получаем компоненты
    let year = zoned.year();
    let month = zoned.month();
    let day = zoned.day();
    let hour = zoned.hour();
    let minute = zoned.minute();
    let second = zoned.second();
    let millisecond = zoned.millisecond(); // миллисекунды 0-999

    // Форматируем через itoa
    dst.extend_from_slice(rendbuf.format(year).as_bytes());
    dst.push(b'-');

    if month < 10 {
        dst.push(b'0');
    }
    dst.extend_from_slice(rendbuf.format(month).as_bytes());
    dst.push(b'-');

    if day < 10 {
        dst.push(b'0');
    }
    dst.extend_from_slice(rendbuf.format(day).as_bytes());
    dst.push(b' ');

    if hour < 10 {
        dst.push(b'0');
    }
    dst.extend_from_slice(rendbuf.format(hour).as_bytes());
    dst.push(b':');

    if minute < 10 {
        dst.push(b'0');
    }
    dst.extend_from_slice(rendbuf.format(minute).as_bytes());
    dst.push(b':');

    if second < 10 {
        dst.push(b'0');
    }
    dst.extend_from_slice(rendbuf.format(second).as_bytes());

    // Миллисекунды с ведущими нулями (3 знака)
    dst.push(b'.');
    if millisecond < 100 {
        dst.push(b'0');
        if millisecond < 10 {
            dst.push(b'0');
        }
    }
    dst.extend_from_slice(rendbuf.format(millisecond).as_bytes());
}

#[inline(always)]
pub(crate) fn render_go_duration(rendbuf: &mut itoa::Buffer, dst: &mut Vec<u8>, nanos: u64) {
    if nanos == 0 {
        dst.extend_from_slice(b"0s");
        return;
    }

    if nanos < 1_000 {
        // < 1µs
        let val = rendbuf.format(nanos);
        dst.extend_from_slice(val.as_bytes());
        dst.extend_from_slice(b"ns");
    } else if nanos < 1_000_000 {
        // < 1ms
        let val = rendbuf.format(nanos / 1_000);
        dst.extend_from_slice(val.as_bytes());
        dst.extend_from_slice(b"\xB5s");
    } else if nanos < 1_000_000_000 {
        // < 1s
        let val = rendbuf.format(nanos / 1_000_000);
        dst.extend_from_slice(val.as_bytes());
        dst.extend_from_slice(b"ms");
    } else {
        let mut seconds = nanos / 1_000_000_000;
        let n = nanos % 1_000_000_000;

        let hours = seconds / 3600;
        seconds %= 3600;
        let minutes = seconds / 60;
        seconds %= 60;

        if hours > 0 {
            let val = rendbuf.format(hours);
            dst.extend_from_slice(val.as_bytes());
            dst.extend_from_slice(b"h");
        }
        if minutes > 0 {
            let val = rendbuf.format(minutes);
            dst.extend_from_slice(val.as_bytes());
            dst.extend_from_slice(b"m");
        }
        if seconds > 0 || n > 0 {
            if n == 0 {
                let val = rendbuf.format(seconds);
                dst.extend_from_slice(val.as_bytes());
                dst.extend_from_slice(b"s");
            } else {
                // Формат секунд с дробной частью, как в Go (до 9 знаков)
                let val = rendbuf.format(seconds);
                dst.extend_from_slice(val.as_bytes());
                let val = rendbuf.format(1_000_000_000 + n);
                let fraction = &val.as_bytes()[1..];
                let mut end = 9;
                while end > 0 && fraction[end - 1] == b'0' {
                    end -= 1;
                }
                if end > 0 {
                    dst.push(b'.');
                    dst.extend_from_slice(&fraction[..end]);
                }
                dst.push(b's');
            }
        }
    }
}
