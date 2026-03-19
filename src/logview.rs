#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::value_kind::{
    PREDEFINED_KEYS, PREDEFINED_NAME_CONTEXT, PREDEFINED_NAME_LOCATION, PREDEFINED_NAME_TEXT,
    ValueKind,
};
use crate::{level, value_kind};
use base64::Engine;
use memchr::{Memchr, Memchr3};
use std::fmt::{Display, Formatter};
use std::io::Read;
use std::io::Write;
use std::slice;

pub struct LogRecord<'a> {
    itoa:      itoa::Buffer,
    ryu:       ryu::Buffer,
    buf:       Vec<u8>,
    need_tree: bool,
    err_stack: Vec<(usize, usize)>,
    grp_stack: Vec<(usize, RenderGroupType)>,

    time:  u64,
    level: u8,
    loc:   Option<(usize, usize, usize)>,
    msg:   (usize, usize),
    ctx:   &'a [u8],
}

impl<'a> LogRecord<'a> {
    pub fn new() -> Self {
        Self {
            itoa:      itoa::Buffer::new(),
            ryu:       ryu::Buffer::new(),
            buf:       Vec::with_capacity(4096),
            need_tree: false,
            err_stack: Vec::with_capacity(8),
            grp_stack: Vec::with_capacity(8),
            time:      0,
            level:     0,
            loc:       None,
            msg:       (0, 0),
            ctx:       &[],
        }
    }

    pub unsafe fn render(&mut self, dst: &mut Vec<u8>, src: &[u8]) {
        let ptr: *const u8 = src.as_ptr();

        render_time(dst, self.time as i64);
        let is_panic = self.write_level(dst);
        self.write_location(dst, ptr);

        if !is_panic {
            let (length, off) = self.msg;
            dst.extend_from_slice(slice::from_raw_parts(ptr.add(off), length));
            dst.push(b' ');

            if self.need_tree {
                self.render_tree(dst, src)
            } else {
                self.render_json(dst, src);
            }
            return;
        }

        self.render_json(dst, src);
        self.render_stacktrace(dst, src);
    }

    pub unsafe fn render_tree(&mut self, dst: &mut Vec<u8>, src: &[u8]) {}

    pub unsafe fn render_json(&mut self, dst: &mut Vec<u8>, src: &[u8]) {
        if src.is_empty() {
            dst.extend_from_slice(b"{}\n");
            return;
        }

        dst.push(b'{');
        let node_ptr = self.ctx.as_ptr();
        let ptr = src.as_ptr();
        let mut pos = 0 as usize;
        let mut old = false;
        let mut is_embed_err = false;
        let mut embed_err_text = (0 as usize, 0 as usize);
        let mut render_state = RenderGroupType::Root;

        'outer: loop {
            if old {
                dst.push(b',');
                dst.push(b' ');
            }
            old = true;

            let node = &*(node_ptr.add(pos) as *const Node);

            match node.kind {
                NodeKind::ErrorStageNew => {
                    dst.push(b'"');
                    dst.extend_from_slice(b"NEW: ");
                    push_json_string_content(dst, node.key_as_slice(ptr));
                    dst.push(b'"');
                }
                NodeKind::ErrorStageWrap => {
                    dst.push(b'"');
                    dst.extend_from_slice(b"WRAP: ");
                    push_json_string_content(dst, node.key_as_slice(ptr));
                    dst.push(b'"');
                }
                NodeKind::ErrorStageCtx => {
                    dst.push(b'"');
                    dst.extend_from_slice(b"CTX");
                    dst.push(b'"');
                }
                _ => {
                    push_json_string(dst, node.key_as_slice(ptr));
                }
            }

            dst.push(b':');
            dst.push(b' ');

            match node.kind {
                NodeKind::Bool => {
                    if node.val_off != 0 {
                        dst.extend_from_slice(b"true");
                    } else {
                        dst.extend_from_slice(b"false");
                    }
                }
                NodeKind::Time => {
                    dst.push(b'"');
                    render_time(dst, node.val_as_u64() as i64);
                    dst.push(b'"');
                }
                NodeKind::Dur => {
                    dst.push(b'"');
                    render_go_duration(dst, node.val_as_u64());
                    dst.push(b'"');
                }
                NodeKind::Int => {
                    self.render_int(dst, node.val_as_u64() as i64);
                }
                NodeKind::I8 => {
                    self.render_int(dst, node.val_off as i64);
                }
                NodeKind::I16 => {
                    self.render_int(dst, node.val_off as i64);
                }
                NodeKind::I32 => {
                    self.render_int(dst, node.val_off as i64);
                }
                NodeKind::I64 => {
                    self.render_int(dst, node.val_as_u64() as i64);
                }
                NodeKind::Uint => {
                    self.render_uint(dst, node.val_as_u64());
                }
                NodeKind::U8 => {
                    self.render_uint(dst, node.val_off as u64);
                }
                NodeKind::U16 => {
                    self.render_uint(dst, node.val_off as u64);
                }
                NodeKind::U32 => {
                    self.render_uint(dst, node.val_off as u64);
                }
                NodeKind::U64 => {
                    self.render_uint(dst, node.val_as_u64());
                }
                NodeKind::F32 => {
                    self.render_float(dst, f32::from_bits(node.val_off) as f64);
                }
                NodeKind::F64 => {
                    self.render_float(dst, f64::from_bits(node.val_as_u64()));
                }
                NodeKind::Str => {
                    push_json_string(dst, node.val_as_slice(ptr));
                }
                NodeKind::Bytes => {
                    dst.push(b'"');
                    dst.extend_from_slice("base64.".as_bytes());
                    base64::engine::general_purpose::STANDARD
                        .encode_slice(node.val_as_slice(ptr), dst);
                    dst.push(b'"');
                }
                NodeKind::ErrTxt => {
                    push_json_string(dst, node.val_as_slice(ptr));
                }
                NodeKind::ErrTxtFragment => {
                    if !is_embed_err {
                        self.err_stack
                            .push((node.val_len as usize, node.val_off as usize));
                    }
                }
                NodeKind::ErrLoc => {
                    dst.push(b'"');
                    dst.extend_from_slice(node.key_as_slice(ptr));
                    dst.push(b':');
                    self.render_uint(dst, node.val_off as u64);
                    dst.push(b'"');
                }
                NodeKind::ErrEmbedText => {
                    embed_err_text = (node.val_len as usize, node.val_off as usize);
                }
                NodeKind::Bools => {
                    dst.push(b'[');
                    for i in 0..node.val_len {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        if *ptr.add(node.val_off as usize + i as usize) != 0 {
                            dst.extend_from_slice(b"true");
                        } else {
                            dst.extend_from_slice(b"false");
                        }
                    }
                    dst.push(b']');
                }
                NodeKind::Ints => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 8).cast::<i64>().read_unaligned();
                        self.render_int(dst, i64::from_le(val));
                    }
                    dst.push(b']');
                }
                NodeKind::I8s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = *xptr.add(i) as i64;
                        self.render_int(dst, val);
                    }
                    dst.push(b']');
                }
                NodeKind::I16s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 2).cast::<i16>().read_unaligned();
                        self.render_int(dst, i16::from_le(val) as i64);
                    }
                    dst.push(b']');
                }
                NodeKind::I32s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 4).cast::<i32>().read_unaligned();
                        self.render_int(dst, i32::from_le(val) as i64);
                    }
                    dst.push(b']');
                }
                NodeKind::I64s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 8).cast::<i64>().read_unaligned();
                        self.render_int(dst, i64::from_le(val));
                    }
                    dst.push(b']');
                }
                NodeKind::Uints => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 8).cast::<u64>().read_unaligned();
                        self.render_uint(dst, u64::from_le(val));
                    }
                    dst.push(b']');
                }
                NodeKind::U8s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = *xptr.add(i) as u64;
                        self.render_uint(dst, val);
                    }
                    dst.push(b']');
                }
                NodeKind::U16s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 2).cast::<u16>().read_unaligned();
                        self.render_uint(dst, u16::from_le(val) as u64);
                    }
                    dst.push(b']');
                }
                NodeKind::U32s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add((i * 4)).cast::<u32>().read_unaligned();
                        self.render_uint(dst, u32::from_le(val) as u64);
                    }
                    dst.push(b']');
                }
                NodeKind::U64s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add((i * 8)).cast::<u64>().read_unaligned();
                        self.render_uint(dst, u64::from_le(val));
                    }
                    dst.push(b']');
                }
                NodeKind::F32s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 4).cast::<u32>().read_unaligned();
                        self.render_float(dst, f32::from_bits(u32::from_le(val)) as f64);
                    }
                    dst.push(b']');
                }
                NodeKind::F64s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = ptr.add(i * 8).cast::<u64>().read_unaligned();
                        self.render_float(dst, f64::from_bits(u64::from_le(val)));
                    }
                    dst.push(b']');
                }
                NodeKind::Strs => {
                    dst.push(b'[');
                    let mut off = node.val_off as usize;
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let (length, size) = read_uvarint(ptr.add(off));
                        let val = slice::from_raw_parts(ptr.add(off + size), length as usize);
                        push_json_string(dst, val);
                        off += size + length as usize;
                    }
                }
                NodeKind::Group => {
                    if node.child != u32::MAX {
                        self.grp_stack.push((pos, render_state));
                        render_state = RenderGroupType::Group;
                        dst.push(b'{');
                        old = false;
                        pos = node.child as usize;
                        continue;
                    } else {
                        dst.extend_from_slice(b"{}");
                    }
                }
                NodeKind::Error => {
                    if node.child != u32::MAX {
                        is_embed_err = false;
                        self.err_stack.clear();
                        self.grp_stack.push((pos, render_state));
                        render_state = RenderGroupType::Error;
                        dst.push(b'{');
                        push_json_string(dst, predefined_key(PREDEFINED_NAME_CONTEXT));
                        dst.push(b':');
                        dst.push(b' ');
                        dst.push(b'{');
                        old = false;
                        pos = node.child as usize;
                        continue;
                    } else {
                        dst.extend_from_slice(b"{}");
                    }
                }
                NodeKind::ErrorEmbed => {
                    if node.child != u32::MAX {
                        is_embed_err = true;
                        self.err_stack.clear();
                        self.grp_stack.push((pos, render_state));
                        render_state = RenderGroupType::ErrorEmbed;
                        dst.push(b'{');
                        push_json_string(dst, predefined_key(PREDEFINED_NAME_CONTEXT));
                        dst.push(b':');
                        dst.push(b' ');
                        dst.push(b'{');
                        old = false;
                        pos = node.child as usize;
                        continue;
                    } else {
                        dst.extend_from_slice(b"{}");
                    }
                }
                NodeKind::ErrorStageNew | NodeKind::ErrorStageWrap | NodeKind::ErrorStageCtx => {
                    if node.child != u32::MAX {
                        self.grp_stack.push((pos, render_state));
                        render_state = RenderGroupType::ErrorStage;
                        dst.push(b'{');
                        old = false;
                        pos = node.child as usize;
                        if node.kind != NodeKind::ErrorStageCtx {
                            self.err_stack
                                .push((node.val_len as usize, node.val_off as usize));
                        }
                        continue;
                    } else {
                        dst.extend_from_slice(b"{}");
                    }
                }
            }

            loop {
                pos = node.next as usize;
                if pos != 0 {
                    continue 'outer;
                }

                if self.grp_stack.is_empty() {
                    break 'outer;
                }
                dst.push(b'}');
                match render_state {
                    RenderGroupType::Error => {
                        dst.push(b',');
                        dst.push(b' ');
                        push_json_string(dst, predefined_key(PREDEFINED_NAME_TEXT));
                        dst.push(b':');
                        dst.push(b' ');
                        dst.push(b'"');
                        for (i, (len, off)) in self.err_stack.iter().rev().enumerate() {
                            if i != 0 {
                                dst.push(b':');
                                dst.push(b' ');
                            }
                            dst.extend_from_slice(slice::from_raw_parts(ptr.add(*off), *len));
                        }
                        dst.push(b'"');
                        dst.push(b'}');
                    }
                    RenderGroupType::ErrorEmbed => {
                        dst.push(b',');
                        dst.push(b' ');
                        push_json_string(dst, predefined_key(PREDEFINED_NAME_TEXT));
                        dst.push(b':');
                        dst.push(b' ');
                        let (length, off) = embed_err_text;
                        let txt = slice::from_raw_parts(ptr.add(off), length);
                        push_json_string(dst, txt);
                        dst.push(b',');
                        dst.push(b' ');
                        dst.push(b'}');
                    }
                    _ => {}
                }
                (pos, render_state) = self.grp_stack.pop().unwrap();
            }
        }

        dst.push(b'}');
        dst.push(b'\n');
    }

    pub unsafe fn render_stacktrace(&mut self, dst: &mut Vec<u8>, src: &[u8]) {
        let ptr: *const u8 = src.as_ptr();
        let (length, off) = self.msg;
        let st = slice::from_raw_parts(ptr.add(off), length);

        let mut decoder = flate2::read::GzDecoder::new(st);
        self.buf.set_len(0);
        match decoder.read_to_end(&mut self.buf) {
            Ok(_) => {}
            Err(x) => {
                self.buf.set_len(0);
                self.buf.extend_from_slice(x.to_string().as_bytes());
            }
        };
        let mut start: usize = 0;
        let haystack = self.buf.as_slice();
        for pos in Memchr::new(b'\n', haystack) {
            dst.extend_from_slice(b".... ");
            dst.extend_from_slice(&haystack[start..pos]);
            dst.push(b'\n');
            start = pos + 1;
        }
        if start < haystack.len() {
            dst.extend_from_slice(b".... ");
            dst.extend_from_slice(&haystack[start..]);
            dst.push(b'\n');
        }
    }

    #[inline(always)]
    unsafe fn write_location(&mut self, dst: &mut Vec<u8>, ptr: *const u8) {
        match self.loc {
            Some((lenght, off, line)) => {
                dst.extend_from_slice(slice::from_raw_parts(ptr.add(off), lenght));
                dst.push(b':');
                let lin = self.itoa.format(line);
                dst.extend_from_slice(lin.as_bytes());
                dst.push(b')');
                dst.push(b' ');
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn write_level(&mut self, dst: &mut Vec<u8>) -> bool {
        let mut is_panic = false;

        match self.level {
            level::TRACE => {
                dst.extend_from_slice(b"TRACE");
            }

            level::DEBUG => {
                dst.extend_from_slice(b"DEBUG");
            }

            level::INFO => {
                dst.extend_from_slice(b" INFO");
            }

            level::WARN => {
                dst.extend_from_slice(b" WARN");
            }

            level::ERROR => {
                dst.extend_from_slice(b"ERROR");
            }

            level::PANIC => {
                is_panic = true;
                dst.extend_from_slice(b"PANIC");
            }

            _ => {
                dst.copy_from_slice(b"     ");
                match self.level {
                    0..10 => {
                        dst.extend_from_slice(b"   ");
                        dst.push(b'!');
                        dst.push('0' as u8 + self.level);
                    }
                    10..100 => {
                        dst.extend_from_slice(b"  ");
                        dst.push(b'!');
                        dst.push(b'0' as u8 + self.level / 10);
                        dst.push(b'0' as u8 + self.level % 10);
                    }
                    100..=255 => {
                        dst.extend_from_slice(b" ");
                        dst.push(b'!');
                        dst.push(b'0' as u8 + self.level / 100);
                        dst.push(b'0' as u8 + (self.level % 100) / 10);
                        dst.push(b'0' as u8 + self.level % 10);
                    }
                }
            }
        }

        dst.push(b' ');
        is_panic
    }

    #[inline(always)]
    fn render_int(&mut self, dst: &mut Vec<u8>, value: i64) {
        let res = self.itoa.format(value);
        dst.extend_from_slice(res.as_bytes());
    }

    #[inline(always)]
    fn render_uint(&mut self, dst: &mut Vec<u8>, value: u64) {
        let res = self.itoa.format(value);
        dst.extend_from_slice(res.as_bytes());
    }

    #[inline(always)]
    fn render_float(&mut self, dst: &mut Vec<u8>, value: f64) {
        let res = self.ryu.format(value);
        dst.extend_from_slice(res.as_bytes());
    }
}

#[inline(always)]
fn render_time(dst: &mut Vec<u8>, unix_nanos: i64) {
    let secs = unix_nanos / 1_000_000_000;
    let nanos = unix_nanos % 1_000_000_000;

    // Математика эпохи (UTC)
    let z = (secs / 86400) + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i32) + (era as i32 * 400);
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + (if m <= 2 { 1 } else { 0 });

    let h = (secs / 3600) % 24;
    let min = (secs / 60) % 60;
    let s = secs % 60;

    // Пишем в буфер "вручную" (без парсинга формат-строки)
    // ГГГГ-ММ-ДД ЧЧ:ММ:СС.нннннн
    let start = dst.len();
    dst.extend_from_slice(b"0000-00-00 00:00:00.000");
    let b = &mut dst[start..];

    // Быстрая запись чисел (можно еще быстрее через lookup-таблицу на 100 байт)
    fn u2(b: &mut [u8], v: i64) {
        b[0] = b'0' + (v / 10 % 10) as u8;
        b[1] = b'0' + (v % 10) as u8;
    }

    // Год
    let y_u = year as i64;
    u2(&mut b[0..2], y_u / 100);
    u2(&mut b[2..4], y_u);
    // Остальное
    u2(&mut b[5..7], m as i64);
    u2(&mut b[8..10], d as i64);
    u2(&mut b[11..13], h);
    u2(&mut b[14..16], min);
    u2(&mut b[17..19], s);

    // Наносекунды (микро)
    let mic = nanos / 1000;
    u2(&mut b[20..23], mic / 1000);
}

#[inline(always)]
fn render_go_duration(dst: &mut Vec<u8>, nanos: u64) {
    if nanos == 0 {
        dst.extend_from_slice(b"0s");
        return;
    }

    if nanos < 1_000 {
        // < 1µs
        write!(dst, "{}ns", nanos).unwrap();
    } else if nanos < 1_000_000 {
        // < 1ms
        write!(dst, "{}µs", nanos / 1_000).unwrap();
    } else if nanos < 1_000_000_000 {
        // < 1s
        write!(dst, "{}ms", nanos / 1_000_000).unwrap();
    } else {
        let mut seconds = nanos / 1_000_000_000;
        let n = nanos % 1_000_000_000;

        let hours = seconds / 3600;
        seconds %= 3600;
        let minutes = seconds / 60;
        seconds %= 60;

        if hours > 0 {
            write!(dst, "{}h", hours).unwrap();
        }
        if minutes > 0 {
            write!(dst, "{}m", minutes).unwrap();
        }
        if seconds > 0 || n > 0 {
            if n == 0 {
                write!(dst, "{}s", seconds).unwrap();
            } else {
                // Формат секунд с дробной частью, как в Go (до 9 знаков)
                let s = format!("{}.{:09}", seconds, n);
                let trimmed = s.trim_end_matches('0').trim_end_matches('.');
                write!(dst, "{}s", trimmed).unwrap();
            }
        }
    }
}

#[inline(always)]
unsafe fn render_err_ctx_header(dst: &mut Vec<u8>) {
    push_json_string(dst, predefined_key(PREDEFINED_NAME_CONTEXT));
    dst.push(b':');
    dst.push(b' ');
    dst.push(b'{');
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum RenderGroupType {
    Root,
    Group,
    Error,
    ErrorEmbed,
    ErrorStage,
}

pub struct LogParser {
    max_log_size:        usize,
    groups_lens:         Vec<(usize, usize)>,
    caps:                Vec<usize>,
    err_frags:           Vec<(usize, usize)>,
    state_stack:         Vec<CtxParsingState>,
    ctx_size:            usize,
    has_errors:          bool,
    use_tree_since:      usize,
    process_since_level: u8,

    // Parsing data.
    time:     u64,
    level:    u8,
    location: Option<(usize, usize, usize)>,
    msg:      (usize, usize),
    ctx:      TreeBuilder,
}

impl LogParser {
    pub fn new() -> Self {
        Self {
            max_log_size:        1 * 1024 * 1024,
            groups_lens:         Vec::with_capacity(8),
            caps:                Vec::with_capacity(8),
            err_frags:           Vec::with_capacity(8),
            state_stack:         Vec::with_capacity(8),
            ctx_size:            0,
            has_errors:          false,
            use_tree_since:      10,
            process_since_level: 0,

            time:     0,
            level:    0,
            location: None,
            msg:      (0, 0),
            ctx:      TreeBuilder::new(),
        }
    }

    pub fn make_record<'a>(&'a self, dst: &'a mut LogRecord<'a>) {
        dst.need_tree = self.ctx_size >= self.use_tree_since || self.has_errors;
        dst.time = self.time;
        dst.level = self.level;
        dst.loc = self.location;
        dst.msg = self.msg;
        dst.ctx = self.ctx.ctrl.as_slice();
        dst.grp_stack.clear();
        dst.err_stack.clear();
    }

    pub fn with_max_log_record_size(&mut self, size: usize) -> &mut Self {
        self.max_log_size = size;
        self
    }

    pub fn with_show_tree_after(&mut self, size: usize) -> &mut Self {
        self.use_tree_since = size;
        self
    }

    pub fn with_show_since_level(&mut self, level: u8) -> &mut Self {
        self.process_since_level = level;
        self
    }

    pub fn should_pass(&self) -> bool {
        self.level < self.process_since_level
    }

    // Parser input source as a log record and returns the rest of data.
    pub unsafe fn parse_log_data<'a>(&mut self, src: &'a [u8]) -> Result<&'a [u8], ErrorLogParse> {
        if src.len() < 5 {
            return Err(ErrorLogParse::NoHeader);
        }

        let ptr = src.as_ptr() as *mut u8;
        if *ptr != 0xFF {
            return Err(ErrorLogParse::StartMarkerInvalid);
        }

        let record_crc = ptr.add(1).cast::<u32>().read_unaligned();
        let (length, size) = read_uvarint_safe(ptr.add(5), src.len() - 5);
        if size == usize::MAX {
            return Err(ErrorLogParse::RecordLengthInvalid);
        }
        if length as usize > self.max_log_size {
            return Err(ErrorLogParse::RecordLengthTooLarge);
        }
        if 5 + size + length as usize > src.len() {
            return Err(ErrorLogParse::RecordNeedMore);
        }
        let off = 5 + size;
        let record = slice::from_raw_parts(ptr.add(off), length as usize);
        let check = crc32c::crc32c(record);
        if check != record_crc {
            return Err(ErrorLogParse::RecordBroken);
        }
        self.parse_log_record(record)?;

        let rest = slice::from_raw_parts(
            ptr.add(off + length as usize),
            src.len() - off - length as usize,
        );
        Ok(rest)
    }

    pub unsafe fn parse_log_record<'a>(&mut self, src: &'a [u8]) -> Result<(), ErrorLogParse> {
        let ptr = src.as_ptr() as *mut u8;

        // Get and check version.
        let version = ptr.cast::<u16>().read_unaligned();
        // TODO
        if version != 1 {
            return Err(ErrorLogParse::RecordVersionNotSupported(version));
        }

        self.time = ptr.add(2).cast::<u64>().read_unaligned();

        self.level = *ptr.add(10);
        if self.level < self.process_since_level {
            return Ok(());
        }
        let mut off: usize = 11;

        self.location = if *ptr.add(off) == 0 {
            off += 1;
            None
        } else {
            let (length, size) = read_uvarint(ptr.add(off));
            let file = slice::from_raw_parts(ptr.add(off + size), length as usize);
            let (line, line_size) = read_uvarint(ptr.add(off + size));
            off += size + length as usize + line_size;
            Some((length as usize, off + size, line as usize))
        };

        let (msg_length, size) = read_uvarint(ptr.add(off));
        self.msg = (msg_length as usize, off + size);
        off += size + msg_length as usize;

        self.parse_ctx(slice::from_raw_parts_mut(ptr.add(off), src.len() - off));

        Ok(())
    }

    pub unsafe fn parse_ctx<'a>(&mut self, src: &'a [u8]) {
        self.ctx.reset();
        self.groups_lens.set_len(0);
        self.caps.set_len(0);
        self.err_frags.set_len(0);
        self.state_stack.set_len(0);
        self.ctx_size = 0;
        self.has_errors = false;

        let mut off: usize = 0;
        let mut need_tree: bool = false;
        let ptr = src.as_ptr() as *mut u8;
        let mut had_stages = false;
        let mut cap = src.len();
        let mut parsing_state = CtxParsingState::Normal;
        let mut group_cap: usize = 0;
        let mut group_off: usize = 0;
        loop {
            match parsing_state {
                CtxParsingState::Normal => {
                    if off >= cap {
                        if self.caps.is_empty() {
                            return;
                        }
                    }
                }

                CtxParsingState::Group => {
                    if group_off >= group_cap {
                        self.ctx.leave_group();
                        if !self.groups_lens.is_empty() {
                            (group_off, group_cap) = self.groups_lens.pop().unwrap();
                            continue;
                        } else {
                            parsing_state = self.state_stack.pop().unwrap();
                        }
                    }
                }

                CtxParsingState::Error => {
                    if off >= cap {
                        self.ctx.leave_group(); // Leave context group which is here.
                        self.ctx.leave_group(); // Leave error itself.
                        if self.caps.is_empty() {
                            return;
                        }
                        cap = self.caps.pop().unwrap();
                        parsing_state = self.state_stack.pop().unwrap();
                    }
                }

                CtxParsingState::ErrorEmbed => {
                    if off >= cap {
                        self.ctx.leave_group();
                        self.ctx.leave_group();
                        if self.caps.is_empty() {
                            return;
                        }
                        cap = self.caps.pop().unwrap();
                        parsing_state = self.state_stack.pop().unwrap();
                    }
                }
            }

            // Read code and continue the loop on some types that have no payload.
            let kind = *(ptr.add(off)) as value_kind::ValueKind;
            off += 1;
            match kind {
                value_kind::JUST_CONTEXT_NODE | value_kind::JUST_CONTEXT_INHERITED_NODE => {
                    had_stages = self.leave_stage_group_if_needed(had_stages);
                    self.ctx.add(NodeKind::ErrorStageCtx, 0, 0, 0, 0);
                    self.ctx.enter_group();
                    continue;
                }
                value_kind::PHANTOM_CONTEXT_NODE => {
                    continue;
                }
                _ => {}
            }

            // Read the key. It can be either 0-lead uvarint of predefined key index, or
            // a literal key with uvarint(length) + body.
            let mut key_len: u32 = 0;
            let mut key_off: u32 = 0;
            let v = *(ptr.add(off));
            if v != 0 {
                let (length, size) = read_uvarint(ptr.add(off));
                key_len = length as u32;
                key_off = (off + size) as u32;
                off += size + length as usize;
            } else {
                let (length, size) = read_uvarint(ptr.add(off + 1));
                key_len = 0;
                key_off = length as u32;
                off += size + 1;
            }

            match kind {
                value_kind::NEW_NODE => {
                    had_stages = self.leave_stage_group_if_needed(had_stages);
                    let (length, size) = read_uvarint(ptr.add(off));
                    self.ctx.add(
                        NodeKind::ErrorStageNew,
                        key_len,
                        key_off,
                        length as u32,
                        (off + size) as u32,
                    );
                    off += size + length as usize;
                    self.ctx.enter_group();
                }
                value_kind::WRAP_NODE | value_kind::WRAP_INHERITED_NODE => {
                    had_stages = self.leave_stage_group_if_needed(had_stages);
                    let (length, size) = read_uvarint(ptr.add(off));
                    self.ctx.add(
                        NodeKind::ErrorStageWrap,
                        key_len,
                        key_off,
                        length as u32,
                        (off + size) as u32,
                    );
                    off += size + length as usize;
                    self.ctx.enter_group();
                }
                value_kind::LOCATION_NODE => {
                    let (length, size) = read_uvarint(ptr.add(off));
                    off += size;
                    self.ctx
                        .add(NodeKind::ErrLoc, key_len, key_off, 0, length as u32);
                }
                value_kind::FOREIGN_ERROR_TEXT => {
                    self.ctx
                        .add(NodeKind::ErrTxtFragment, key_len, key_off, 0, 0);
                }
                value_kind::FOREIGN_ERROR_FORMAT => {
                    // Not supported as for now.
                }

                value_kind::BOOL => {
                    let v = *(ptr.add(off));
                    self.ctx.add(NodeKind::Bool, key_len, key_off, 0, v as u32);
                    off += 1;
                    self.ctx_size += 1;
                }

                value_kind::TIME => {
                    let v = *(ptr.add(off) as *const u64);
                    self.ctx
                        .add(NodeKind::Time, key_len, key_off, v as u32, (v >> 32) as u32);
                    off += 8;
                    self.ctx_size += 1;
                }

                value_kind::DURATION => {
                    let v = *(ptr.add(off) as *const u64);
                    self.ctx
                        .add(NodeKind::Time, key_len, key_off, v as u32, (v >> 32) as u32);
                    off += 8;
                    self.ctx_size += 1;
                }

                value_kind::I => {
                    let v = *(ptr.add(off) as *const u64);
                    self.ctx
                        .add(NodeKind::Int, key_len, key_off, v as u32, (v >> 32) as u32);
                    off += 8;
                    self.ctx_size += 1;
                }

                value_kind::I8 => {
                    let v = *(ptr.add(off) as *const i8);
                    self.ctx.add(NodeKind::I8, key_len, key_off, 0, v as u32);
                    off += 1;
                    self.ctx_size += 1;
                }

                value_kind::I16 => {
                    let v = *(ptr.add(off) as *const i16);
                    self.ctx.add(NodeKind::I16, key_len, key_off, 0, v as u32);
                    off += 2;
                    self.ctx_size += 1;
                }

                value_kind::I32 => {
                    let v = *(ptr.add(off) as *const i32);
                    self.ctx.add(NodeKind::I32, key_len, key_off, 0, v as u32);
                    off += 4;
                    self.ctx_size += 1;
                }

                value_kind::I64 => {
                    let v = *(ptr.add(off) as *const i64);
                    self.ctx
                        .add(NodeKind::Int, key_len, key_off, v as u32, (v >> 32) as u32);
                    off += 8;
                    self.ctx_size += 1;
                }

                value_kind::U => {
                    let v = *(ptr.add(off) as *const u64);
                    self.ctx
                        .add(NodeKind::Uint, key_len, key_off, v as u32, (v >> 32) as u32);
                    off += 8;
                    self.ctx_size += 1;
                }

                value_kind::U8 => {
                    let v = *(ptr.add(off) as *const u8);
                    self.ctx.add(NodeKind::U8, key_len, key_off, 0, v as u32);
                    off += 1;
                    self.ctx_size += 1;
                }

                value_kind::U16 => {
                    let v = *(ptr.add(off) as *const u16);
                    self.ctx.add(NodeKind::U16, key_len, key_off, 0, v as u32);
                    off += 2;
                    self.ctx_size += 1;
                }

                value_kind::U32 => {
                    let v = *(ptr.add(off) as *const u32);
                    self.ctx.add(NodeKind::U32, key_len, key_off, 0, v);
                    off += 4;
                    self.ctx_size += 1;
                }

                value_kind::U64 => {
                    let v = *(ptr.add(off) as *const u64);
                    self.ctx
                        .add(NodeKind::U64, key_len, key_off, v as u32, (v >> 32) as u32);
                    off += 8;
                    self.ctx_size += 1;
                }

                value_kind::FLOAT32 => {
                    let v = *(ptr.add(off) as *const u32);
                    self.ctx.add(NodeKind::F32, key_len, key_off, 0, v);
                    off += 4;
                    self.ctx_size += 1;
                }

                value_kind::FLOAT64 => {
                    let v = *(ptr.add(off) as *const u64);
                    self.ctx
                        .add(NodeKind::F64, key_len, key_off, v as u32, (v >> 32) as u32);
                    off += 8;
                    self.ctx_size += 1;
                }

                value_kind::STRING => {
                    off = self.varthing(ptr, off, NodeKind::Str, key_len, key_off);
                    self.ctx_size += 1;
                }

                value_kind::BYTES => {
                    off = self.varthing(ptr, off, NodeKind::Bytes, key_len, key_off);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_BOOL => {
                    off = self.slice(ptr, off, NodeKind::Bool, key_len, key_off, 1);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_I => {
                    off = self.slice(ptr, off, NodeKind::Ints, key_len, key_off, 8);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_I8 => {
                    off = self.slice(ptr, off, NodeKind::I8s, key_len, key_off, 1);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_I16 => {
                    off = self.slice(ptr, off, NodeKind::I16s, key_len, key_off, 2);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_I32 => {
                    off = self.slice(ptr, off, NodeKind::I32s, key_len, key_off, 4);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_I64 => {
                    off = self.slice(ptr, off, NodeKind::I64s, key_len, key_off, 8);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_U => {
                    off = self.slice(ptr, off, NodeKind::Uints, key_len, key_off, 8);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_U8 => {
                    off = self.slice(ptr, off, NodeKind::U8s, key_len, key_off, 1);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_U16 => {
                    off = self.slice(ptr, off, NodeKind::U16s, key_len, key_off, 2);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_U32 => {
                    off = self.slice(ptr, off, NodeKind::U32s, key_len, key_off, 4);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_U64 => {
                    off = self.slice(ptr, off, NodeKind::U64s, key_len, key_off, 8);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_F32 => {
                    off = self.slice(ptr, off, NodeKind::F32s, key_len, key_off, 4);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_F64 => {
                    off = self.slice(ptr, off, NodeKind::F64s, key_len, key_off, 8);
                    self.ctx_size += 1;
                }

                value_kind::SLICE_STRING => {
                    let (lenght, size) = read_uvarint(ptr.add(off));
                    off += size;
                    let start = off;
                    for _ in 0..lenght {
                        let (length, size) = read_uvarint(ptr.add(off));
                        off += size + length as usize;
                    }
                    self.ctx.add(
                        NodeKind::Strs,
                        key_len,
                        key_off,
                        lenght as u32,
                        start as u32,
                    );
                    self.ctx_size += 1;
                }

                value_kind::ERROR => {
                    self.ctx.add(NodeKind::Error, key_len, key_off, 0, 0);
                    self.ctx.enter_group();
                    let (lenght, size) = read_uvarint(ptr.add(off));
                    off += size;
                    self.caps.push(cap);
                    self.state_stack.push(parsing_state);
                    parsing_state = CtxParsingState::Error;
                    cap = off + lenght as usize;
                    self.ctx_size += 1;
                    self.has_errors = true;
                }

                value_kind::ERROR_EMBED => {
                    self.ctx.add(NodeKind::ErrorEmbed, key_len, key_off, 0, 0);
                    self.ctx.enter_group();
                    let (lenght, size) = read_uvarint(ptr.add(off));
                    off += size;
                    self.ctx
                        .add(NodeKind::ErrEmbedText, 0, 0, lenght as u32, off as u32);
                    self.caps.push(cap);
                    self.state_stack.push(parsing_state);
                    off = off + lenght as usize;
                    let (lenght, size) = read_uvarint(ptr.add(off));
                    cap = off + size + lenght as usize;
                    self.ctx_size += 1;
                    self.has_errors = true;
                }

                value_kind::GROUP => {
                    self.ctx.add(NodeKind::Group, key_len, key_off, 0, 0);
                    let (lenght, size) = read_uvarint(ptr.add(off));
                    off += size;
                    self.groups_lens.push((group_off, group_cap));
                    self.state_stack.push(parsing_state);
                    group_off = 0;
                    group_cap = lenght as usize;
                    parsing_state = CtxParsingState::Group;
                    self.ctx_size += 1;
                }

                _ => {}
            }

            group_off += 1;
        }
    }

    #[inline(always)]
    pub unsafe fn leave_stage_group_if_needed(&mut self, had_stages: bool) -> bool {
        if had_stages {
            self.ctx.leave_group();
        }

        true
    }

    #[inline(always)]
    pub unsafe fn varthing(
        &mut self,
        src: *const u8,
        off: usize,
        nkind: NodeKind,
        key_len: u32,
        key_off: u32,
    ) -> usize {
        let (length, size) = read_uvarint(src.add(off));
        self.ctx
            .add(nkind, key_len, key_off, length as u32, (off + size) as u32);

        off + size + length as usize
    }
    #[inline(always)]
    pub unsafe fn slice(
        &mut self,
        src: *const u8,
        off: usize,
        nkind: NodeKind,
        key_len: u32,
        key_off: u32,
        siz: usize,
    ) -> usize {
        let (length, size) = read_uvarint(src.add(off));
        self.ctx
            .add(nkind, key_len, key_off, length as u32, (off + size) as u32);

        off + size + (length as usize) * siz
    }
}

/// Log parsing error states.
#[derive(Copy, Clone, Debug)]
enum ErrorLogParse {
    /// Missing this
    ///
    /// | 0xFF | CRC32 |
    /// |------|-------|
    ///
    /// 5 bytes header.
    NoHeader,
    /// Log data must start with 0xFF, got something different.
    StartMarkerInvalid,
    /// Record length in uvarint encoding is either cut or something is off with it.
    RecordLengthInvalid,
    /// Record length is out of limit.
    RecordLengthTooLarge,
    /// The rest of data does not have the entire record. Need to read more.
    RecordNeedMore,
    /// Record data does not match the CRC.
    RecordBroken,
    /// Record data has unsupported version.
    RecordVersionNotSupported(u16),
    /// Record data has unsupported level.
    RecordLevelNotSupported(u8),
}

/// Denotes a context parsing state.
#[derive(Copy, Clone, Debug)]
enum CtxParsingState {
    Normal,
    Group,
    Error,
    ErrorEmbed,
}

/// Defines a node of blog context structure.
///
/// Here:
///  - `kind` denotes a kind of node.
///  - `next` is u32 offset the next sibling.
///  - `child` is u32 offset of the first child.
///  - `data` is u32 offset to the respective log fragment.
#[repr(C)]
pub struct Node {
    kind:    NodeKind,
    next:    u32,
    child:   u32,
    _pad:    u32,
    key_len: u32,
    key_off: u32,
    val_len: u32,
    val_off: u32,
}

impl Node {
    #[inline(always)]
    pub fn val_as_u64(&self) -> u64 {
        self.val_len as u64 | (self.val_off as u64) << 32
    }

    #[inline(always)]
    pub unsafe fn val_as_slice(&self, ptr: *const u8) -> &[u8] {
        slice::from_raw_parts(ptr.add(self.val_off as usize), self.val_len as usize)
    }

    #[inline(always)]
    pub unsafe fn key_as_slice(&self, ptr: *const u8) -> &[u8] {
        match self.kind {
            NodeKind::ErrLoc => {
                let lock_key_index = PREDEFINED_NAME_LOCATION >> 8 - 1;
                let key = PREDEFINED_KEYS[lock_key_index as usize];
                return key.as_bytes();
            }
            _ => {}
        }

        if self.key_len != 0 {
            return slice::from_raw_parts(ptr.add(self.key_off as usize), self.key_len as usize);
        }

        if self.key_off >= PREDEFINED_KEYS.len() as u32 {
            return "!unknown-key".as_bytes();
        }

        let key = PREDEFINED_KEYS[self.key_len as usize];
        key.as_bytes()
    }
}

const _: () = assert!(std::mem::size_of::<Node>() == 32);

/// Represents a part of kind value in [Node].
/// Splited into three regions:
/// - 0…63 values.
/// - 64…127 slices.
/// - 128…255 hierarchy roots.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    // Values
    Bool           = 0,
    Time           = 1,
    Dur            = 2,
    Int            = 3,
    I8             = 4,
    I16            = 5,
    I32            = 6,
    I64            = 7,
    Uint           = 8,
    U8             = 10,
    U16            = 11,
    U32            = 12,
    U64            = 13,
    F32            = 14,
    F64            = 15,
    Str            = 16,
    Bytes          = 17,
    ErrTxt         = 18,
    ErrTxtFragment = 19,
    ErrLoc         = 20,
    ErrEmbedText   = 21,

    // Slices/arrays/lists or whatever you call them.
    Bools          = 64,
    Ints           = 65,
    I8s            = 66,
    I16s           = 67,
    I32s           = 68,
    I64s           = 69,
    Uints          = 70,
    U8s            = 71,
    U16s           = 72,
    U32s           = 73,
    U64s           = 74,
    F32s           = 75,
    F64s           = 76,
    Strs           = 77,

    // Roots.
    Group          = 128,
    Error          = 129,
    ErrorEmbed     = 130,
    ErrorStageNew  = 131,
    ErrorStageWrap = 132,
    ErrorStageCtx  = 133,
}

impl NodeKind {
    fn string(&self) -> &'static str {
        match self {
            NodeKind::Bool => "bool",
            NodeKind::Time => "time",
            NodeKind::Dur => "dur",
            NodeKind::Int => "int",
            NodeKind::I8 => "i8",
            NodeKind::I16 => "i16",
            NodeKind::I32 => "i32",
            NodeKind::I64 => "i64",
            NodeKind::Uint => "uint",
            NodeKind::U8 => "u8",
            NodeKind::U16 => "u16",
            NodeKind::U32 => "u32",
            NodeKind::U64 => "u64",
            NodeKind::F32 => "f32",
            NodeKind::F64 => "f64",
            NodeKind::Str => "str",
            NodeKind::Bytes => "bytes",
            NodeKind::ErrTxt => "error:Text",
            NodeKind::ErrLoc => "error:Loc",
            NodeKind::ErrEmbedText => "error:EmbedText",
            NodeKind::Bools => "bools",
            NodeKind::Ints => "ints",
            NodeKind::I8s => "i8s",
            NodeKind::I16s => "i16s",
            NodeKind::I32s => "i32s",
            NodeKind::I64s => "i64s",
            NodeKind::Uints => "uints",
            NodeKind::U8s => "u8s",
            NodeKind::U16s => "u16s",
            NodeKind::U32s => "u32s",
            NodeKind::U64s => "u64s",
            NodeKind::F32s => "f32s",
            NodeKind::F64s => "f64s",
            NodeKind::Strs => "strs",
            NodeKind::Group => "group",
            NodeKind::Error => "error",
            NodeKind::ErrorEmbed => "error:embed",
            NodeKind::ErrorStageNew => "error:New",
            NodeKind::ErrorStageWrap => "error:Wrap",
            NodeKind::ErrorStageCtx => "error:Ctx",
            &NodeKind::ErrTxtFragment => "error:TextFragment",
        }
    }
}

/// [Tree] represents a logical tree layout in a continuous memory area.
pub struct Tree<'a> {
    ctrl: &'a [u8],
    tree: bool,
}

/// blog context structure builder type.
pub struct TreeBuilder {
    pub ctrl:  Vec<u8>,
    pub stack: Vec<usize>,
    pub last:  isize,
    pub off:   usize,
}

impl TreeBuilder {
    /// Constructs new [TreeBuilder] instance with preallocated data.
    pub fn new() -> Self {
        Self {
            ctrl:  vec![0u8; 4096],
            stack: Vec::with_capacity(16),
            last:  -1,
            off:   0,
        }
    }

    /// Resets the state of builder for further reuse.
    pub unsafe fn reset(&mut self) {
        self.stack.set_len(0);
        self.last = -1;
        self.off = 0;
    }

    /// Adds a new node with given type and data.
    #[inline(always)]
    pub unsafe fn add(
        &mut self,
        kind: NodeKind,
        key_len: u32,
        key_off: u32,
        val_len: u32,
        val_off: u32,
    ) {
        let node_size = 32;
        let pos = self.off;

        // 1. Check capacity and reallocate if needed.
        if self.ctrl.capacity() < pos + 32 {
            // Get some more space to avoid non-stop reallocates.
            self.ctrl.reserve(1024);
            self.ctrl.set_len(self.ctrl.capacity());
        }

        let ptr = self.ctrl.as_mut_ptr();

        // 3. Get direct pointer to a Node in ctrl.
        let node = ptr.add(pos) as *mut Node;
        (*node).kind = kind;
        (*node).next = 0;
        (*node).child = 0;
        (*node).key_len = key_len;
        (*node).key_off = key_off;
        (*node).val_len = val_len;
        (*node).val_off = val_off;

        if self.last < 0 {
            (self.last, self.off) = (self.off as isize, self.off + node_size);
            return;
        }

        let prev = ptr.add(self.last as usize) as *mut Node;
        let kind: u32 = (*prev).kind as u32;
        if (kind >> 7) > 0 && (*prev).child == 0 {
            (*prev).child = self.off as u32;
        } else {
            (*prev).next = self.off as u32;
        }
        (self.last, self.off) = (self.off as isize, self.off + node_size);
    }

    /// Starts a new group AFTER root node (kind >=128). There's no check though,
    /// this is up to a user.
    /// This call must be paired with respective leave_group.
    #[inline(always)]
    pub fn enter_group(&mut self) {
        self.stack.push(self.last as usize);
    }

    /// Exits existing group. Must be called somewhere after enter_group.
    #[inline(always)]
    pub unsafe fn leave_group(&mut self) {
        let last = self.last;
        self.last = self.stack.pop().unwrap() as isize;

        if last == self.last {
            // We are closing a root node that has no content.
            let node = self.ctrl.as_mut_ptr().add(last as usize) as *mut Node;
            (*node).child = u32::MAX;
        }
    }

    /// Превращает билд в готовое дерево (Zero-copy ссылки)
    pub unsafe fn finish<'a>(&'a self) -> Tree<'a> {
        let res: &[u8] = unsafe {
            let ptr = self.ctrl.as_ptr();
            slice::from_raw_parts(ptr, self.off)
        };
        Tree {
            ctrl: res,
            tree: true,
        }
    }

    // Shows collected data as a dump.
    pub unsafe fn show(&mut self) {
        if self.off == 0 {
            println!("empty");
            return;
        }

        let mut pos = 0;
        let mut stack: Vec<usize> = Vec::with_capacity(16);
        let ptr = self.ctrl.as_mut_ptr();

        loop {
            let node = &mut *(ptr.add(pos) as *mut Node);

            println!(
                "{:03X} {:10} next[{:03X}] child[{:03X}] key.len[{:03}] key.off[{:03}] val[{:03}] val.off[{:03}]",
                pos,
                node.kind.string(),
                node.next,
                node.child,
                node.key_len,
                node.key_off,
                node.val_len,
                node.val_off,
            );

            if (node.kind as u32 >> 7) != 0 && node.child != u32::MAX {
                stack.push(pos);
                pos = node.child as usize;
                continue;
            }

            let mut curr_node = node;
            loop {
                if curr_node.next != 0 {
                    pos = curr_node.next as usize;
                    break;
                }

                if stack.is_empty() {
                    return;
                }

                pos = stack.pop().unwrap();
                curr_node = &mut *(ptr.add(pos) as *mut Node);
            }
        }
    }
}

#[inline(always)]
pub unsafe fn read_uvarint(ptr: *const u8) -> (u64, usize) {
    let mut res = 0u64;
    let mut i = 0;
    loop {
        let b = *ptr.add(i);
        res |= ((b & 0x7F) as u64) << (i * 7);
        i += 1;
        if b & 0x80 == 0 {
            break;
        }
    }

    (res, i)
}

#[inline(always)]
pub unsafe fn read_uvarint_safe(ptr: *const u8, mut lim: usize) -> (u64, usize) {
    let mut res = 0u64;
    let mut i = 0;
    if lim > 10 {
        lim = 10;
    }
    loop {
        if i >= lim {
            return (res, usize::MAX);
        }
        let b = *ptr.add(i);
        res |= ((b & 0x7F) as u64) << (i * 7);
        i += 1;
        if b & 0x80 == 0 {
            break;
        }
    }

    (res, i)
}

#[inline(always)]
pub unsafe fn push_json_string(dst: &mut Vec<u8>, src: &[u8]) {
    dst.push(b'"');
    push_json_string(dst, src);
    dst.push(b'"');
}

pub fn predefined_key<'a>(key: ValueKind) -> &'a [u8] {
    if key < 255 as ValueKind {
        return "!invalid-predefined-key".as_bytes();
    }

    let index = key >> 8 - 1;
    if index >= PREDEFINED_KEYS.len() as ValueKind {
        return "!unknown-predefined-key".as_bytes();
    }

    let value = PREDEFINED_KEYS[index as usize];
    return value.as_bytes();
}

#[inline(always)]
pub unsafe fn push_json_string_content(dst: &mut Vec<u8>, src: &[u8]) {
    let Some(first_escape) = src.iter().position(|&b| NEEDS_ESCAPE[b as usize] != 0) else {
        dst.extend_from_slice(src);
        return;
    };

    let mut start = 0;
    let mut i = first_escape;

    while i < src.len() {
        let b = src[i];

        if NEEDS_ESCAPE[b as usize] == 0 {
            i += 1;
            continue;
        }

        if start < i {
            dst.extend_from_slice(&src[start..i]);
        }

        match b {
            b'"' => dst.extend_from_slice(br#"\""#),
            b'\\' => dst.extend_from_slice(br#"\\"#),
            b'\n' => dst.extend_from_slice(br#"\n"#),
            b'\r' => dst.extend_from_slice(br#"\r"#),
            b'\t' => dst.extend_from_slice(br#"\t"#),
            0x08 => dst.extend_from_slice(br#"\b"#),
            0x0C => dst.extend_from_slice(br#"\f"#),
            _ => {
                // control chars: \u00XX
                dst.extend_from_slice(br#"\u00"#);
                dst.push(HEX[(b >> 4) as usize]);
                dst.push(HEX[(b & 0x0F) as usize]);
            }
        }

        i += 1;
        start = i;
    }

    if start < src.len() {
        dst.extend_from_slice(&src[start..]);
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

const NEEDS_ESCAPE: [u8; 256] = build_needs_escape();

const fn build_needs_escape() -> [u8; 256] {
    let mut t = [0u8; 256];

    let mut i = 0;
    while i < 0x20 {
        t[i] = 1;
        i += 1;
    }

    t[b'"' as usize] = 1;
    t[b'\\' as usize] = 1;

    t
}

#[cfg(test)]
mod test {
    use crate::logview::{NodeKind, TreeBuilder};

    #[test]
    fn test_tree_builder() {
        unsafe {
            let mut b = TreeBuilder::new();
            b.add(NodeKind::Bool, 2, 3, 4, 5);
            b.add(NodeKind::Int, 3, 4, 5, 6);
            b.add(NodeKind::Group, 31, 32, 33, 34);
            b.enter_group();
            b.add(NodeKind::Bool, 32, 33, 34, 35);
            b.add(NodeKind::Int, 33, 34, 35, 36);
            b.add(NodeKind::Group, 331, 332, 333, 333);
            b.enter_group();
            b.add(NodeKind::F32, 332, 333, 334, 335);
            b.leave_group();
            b.add(NodeKind::Str, 35, 36, 37, 38);
            b.leave_group();
            b.add(NodeKind::Bytes, 5, 6, 7, 8);

            println!();
            b.show();
        }
    }
}
