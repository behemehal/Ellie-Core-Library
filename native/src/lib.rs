use ellie_native_bridge::rust::{
    EllieBytesHandle, EllieCtx, EllieData, EllieFunction, EllieHandle, EllieModule,
    FunctionAnswer, FunctionCallParameter, ELLIE_RUST_ABI_VERSION,
};

// ── Argument helpers ──────────────────────────────────────────────────────────

fn str_arg(args: &[FunctionCallParameter], idx: usize) -> String {
    args.get(idx)
        .and_then(|p| match &p.data {
            EllieData::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn int_arg(args: &[FunctionCallParameter], idx: usize) -> isize {
    args.get(idx)
        .and_then(|p| match &p.data {
            EllieData::Integer(i) => Some(i.as_isize),
            _ => None,
        })
        .unwrap_or(0)
}

fn byte_arg(args: &[FunctionCallParameter], idx: usize) -> u8 {
    args.get(idx)
        .and_then(|p| match &p.data {
            EllieData::Byte(b) => Some(*b),
            EllieData::Integer(i) => Some((i.as_isize & 0xff) as u8),
            _ => None,
        })
        .unwrap_or(0)
}

fn bytes_arg(args: &[FunctionCallParameter], idx: usize) -> Option<EllieBytesHandle> {
    args.get(idx).and_then(|p| match &p.data {
        EllieData::Bytes(h) => Some(*h),
        _ => None,
    })
}

fn handle_arg(args: &[FunctionCallParameter], idx: usize) -> Option<EllieHandle> {
    args.get(idx).and_then(|p| match &p.data {
        EllieData::Handle(h) => Some(*h),
        _ => None,
    })
}

fn string_array_arg(args: &[FunctionCallParameter], idx: usize) -> Vec<String> {
    args.get(idx)
        .and_then(|p| match &p.data {
            EllieData::Array(items) => Some(
                items
                    .iter()
                    .filter_map(|item| match item {
                        EllieData::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn ok(data: impl Into<EllieData>) -> FunctionAnswer {
    FunctionAnswer::Ok(data.into())
}

fn ok_bool(b: bool) -> FunctionAnswer {
    FunctionAnswer::Ok(EllieData::Bool(b))
}

fn ok_int(i: isize) -> FunctionAnswer {
    FunctionAnswer::Ok(i.into())
}

fn ok_byte(b: u8) -> FunctionAnswer {
    FunctionAnswer::Ok(EllieData::Byte(b))
}

fn ok_bytes(h: EllieBytesHandle) -> FunctionAnswer {
    FunctionAnswer::Ok(EllieData::Bytes(h))
}

fn ok_handle(h: EllieHandle) -> FunctionAnswer {
    FunctionAnswer::Ok(EllieData::Handle(h))
}

fn ok_null() -> FunctionAnswer {
    FunctionAnswer::Ok(EllieData::Null)
}

fn ok_str_array(items: Vec<String>) -> FunctionAnswer {
    let arr: Vec<EllieData> = items.into_iter().map(EllieData::String).collect();
    FunctionAnswer::Ok(EllieData::Array(arr))
}

// ── platform.ei ───────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn get_arch(_ctx: &mut dyn EllieCtx, _args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    ok(std::env::consts::ARCH)
}

#[no_mangle]
pub extern "Rust" fn get_os(_ctx: &mut dyn EllieCtx, _args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    ok(std::env::consts::OS)
}

#[no_mangle]
pub extern "Rust" fn get_eol(_ctx: &mut dyn EllieCtx, _args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    #[cfg(target_os = "windows")]
    let eol = "\r\n";
    #[cfg(not(target_os = "windows"))]
    let eol = "\n";
    ok(eol)
}

#[no_mangle]
pub extern "Rust" fn print(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let s = str_arg(&args, 0);
    std::print!("{}", s);
    ok(())
}

#[no_mangle]
pub extern "Rust" fn println(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let s = str_arg(&args, 0);
    std::println!("{}", s);
    ok(())
}

#[no_mangle]
pub extern "Rust" fn panic(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let msg = str_arg(&args, 0);
    eprintln!("panic: {}", msg);
    std::process::exit(1);
}

#[no_mangle]
pub extern "Rust" fn timestamp(_ctx: &mut dyn EllieCtx, _args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use std::time::SystemTime;
    let ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as isize;
    ok_int(ms)
}

#[no_mangle]
pub extern "Rust" fn sleep_ms(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let ms = int_arg(&args, 0);
    std::thread::sleep(std::time::Duration::from_millis(ms.max(0) as u64));
    ok(())
}

// ── build.ei — process / shell ────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn command_exists(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let cmd = str_arg(&args, 0);
    let found = if let Some(path_os) = std::env::var_os("PATH") {
        std::env::split_paths(&path_os).any(|dir| {
            if dir.join(&cmd).is_file() {
                return true;
            }
            #[cfg(target_os = "windows")]
            {
                if dir.join(format!("{}.exe", cmd)).is_file()
                    || dir.join(format!("{}.cmd", cmd)).is_file()
                {
                    return true;
                }
            }
            false
        })
    } else {
        false
    };
    ok_bool(found)
}

#[no_mangle]
pub extern "Rust" fn run_command(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let cmd = str_arg(&args, 0);
    let cmd_args = string_array_arg(&args, 1);
    let code = std::process::Command::new(&cmd)
        .args(&cmd_args)
        .status()
        .map(|s| s.code().unwrap_or(-1))
        .unwrap_or(-1);
    ok_int(code as isize)
}

#[no_mangle]
pub extern "Rust" fn run_command_silent(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use std::process::Stdio;
    let cmd = str_arg(&args, 0);
    let cmd_args = string_array_arg(&args, 1);
    let code = std::process::Command::new(&cmd)
        .args(&cmd_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.code().unwrap_or(-1))
        .unwrap_or(-1);
    ok_int(code as isize)
}

#[no_mangle]
pub extern "Rust" fn run_command_output(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let cmd = str_arg(&args, 0);
    let cmd_args = string_array_arg(&args, 1);
    let out = std::process::Command::new(&cmd)
        .args(&cmd_args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    ok(out)
}

// ── build.ei — environment / cwd ─────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn get_env(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let name = str_arg(&args, 0);
    ok(std::env::var(&name).unwrap_or_default())
}

#[no_mangle]
pub extern "Rust" fn set_env(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let name = str_arg(&args, 0);
    let value = str_arg(&args, 1);
    unsafe { std::env::set_var(&name, &value) };
    ok(())
}

#[no_mangle]
pub extern "Rust" fn get_cwd(_ctx: &mut dyn EllieCtx, _args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    ok(cwd)
}

#[no_mangle]
pub extern "Rust" fn set_cwd(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    ok_bool(std::env::set_current_dir(&path).is_ok())
}

// ── build.ei — current process ───────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn process_args(_ctx: &mut dyn EllieCtx, _args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let args: Vec<String> = std::env::args().skip(1).collect();
    ok_str_array(args)
}

#[no_mangle]
pub extern "Rust" fn process_pid(_ctx: &mut dyn EllieCtx, _args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    ok_int(std::process::id() as isize)
}

#[no_mangle]
pub extern "Rust" fn process_exit(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let code = int_arg(&args, 0) as i32;
    std::process::exit(code);
}

// ── fs.ei ─────────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn fs_read_file(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    ok(std::fs::read_to_string(&path).unwrap_or_default())
}

#[no_mangle]
pub extern "Rust" fn fs_write_file(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    let contents = str_arg(&args, 1);
    ok_bool(std::fs::write(&path, contents.as_bytes()).is_ok())
}

#[no_mangle]
pub extern "Rust" fn fs_append_file(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use std::io::Write;
    let path = str_arg(&args, 0);
    let contents = str_arg(&args, 1);
    let result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(contents.as_bytes()));
    ok_bool(result.is_ok())
}

#[no_mangle]
pub extern "Rust" fn fs_exists(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    ok_bool(std::path::Path::new(&path).exists())
}

#[no_mangle]
pub extern "Rust" fn fs_is_file(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    ok_bool(std::path::Path::new(&path).is_file())
}

#[no_mangle]
pub extern "Rust" fn fs_is_dir(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    ok_bool(std::path::Path::new(&path).is_dir())
}

#[no_mangle]
pub extern "Rust" fn fs_remove_file(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    ok_bool(std::fs::remove_file(&path).is_ok())
}

#[no_mangle]
pub extern "Rust" fn fs_remove_dir(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    ok_bool(std::fs::remove_dir(&path).is_ok())
}

#[no_mangle]
pub extern "Rust" fn fs_remove_dir_all(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    ok_bool(std::fs::remove_dir_all(&path).is_ok())
}

#[no_mangle]
pub extern "Rust" fn fs_rename(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let from = str_arg(&args, 0);
    let to = str_arg(&args, 1);
    ok_bool(std::fs::rename(&from, &to).is_ok())
}

#[no_mangle]
pub extern "Rust" fn fs_create_dir(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    ok_bool(std::fs::create_dir(&path).is_ok())
}

#[no_mangle]
pub extern "Rust" fn fs_create_dir_all(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    ok_bool(std::fs::create_dir_all(&path).is_ok())
}

#[no_mangle]
pub extern "Rust" fn fs_read_dir(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    let entries: Vec<String> = match std::fs::read_dir(&path) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    ok_str_array(entries)
}

#[no_mangle]
pub extern "Rust" fn fs_size(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    let size = std::fs::metadata(&path).map(|m| m.len() as isize).unwrap_or(-1);
    ok_int(size)
}

#[no_mangle]
pub extern "Rust" fn fs_modified_ms(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use std::time::UNIX_EPOCH;
    let path = str_arg(&args, 0);
    let ms = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as isize)
        .unwrap_or(-1);
    ok_int(ms)
}

// fs_read_bytes: returns a new bytes value with the file contents.
#[no_mangle]
pub extern "Rust" fn fs_read_bytes(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    let data = std::fs::read(&path).unwrap_or_default();
    ok_bytes(ctx.bytes_new(data))
}

// fs_write_bytes: writes the byte buffer to a file. Returns true on success.
#[no_mangle]
pub extern "Rust" fn fs_write_bytes(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let path = str_arg(&args, 0);
    let h = match bytes_arg(&args, 1) {
        Some(h) => h,
        None => return ok_bool(false),
    };
    let view = match ctx.bytes_view(h) {
        Some(v) => v,
        None => return ok_bool(false),
    };
    ok_bool(std::fs::write(&path, view).is_ok())
}

// fs_read_into: read up to `len` bytes from `path` directly into `buf[offset..]`.
// Returns bytes actually read, or -1 on error.
#[no_mangle]
pub extern "Rust" fn fs_read_into(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use std::io::Read;
    let path = str_arg(&args, 0);
    let h = match bytes_arg(&args, 1) {
        Some(h) => h,
        None => return ok_int(-1),
    };
    let offset = int_arg(&args, 2).max(0) as usize;
    let len = int_arg(&args, 3).max(0) as usize;

    let slice = match ctx.bytes_mut_slice(h, offset, len) {
        Some(s) => s,
        None => return ok_int(-1),
    };

    let mut file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return ok_int(-1),
    };
    match file.read(slice) {
        Ok(n) => ok_int(n as isize),
        Err(_) => ok_int(-1),
    }
}

// ── bytes.ei intrinsics ──────────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn bytes_alloc(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let n = int_arg(&args, 0).max(0) as usize;
    ok_bytes(ctx.bytes_new(vec![0u8; n]))
}

// Processes C-style escapes (`\n` `\r` `\t` `\0` `\\` `\"` `\xNN`) so users
// writing string literals for binary protocols (HTTP requests, etc.) get the
// expected bytes. Raw strings on the Ellie side don't process escapes.
fn decode_escapes(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            // Encode as UTF-8
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        match it.next() {
            Some('n') => out.push(b'\n'),
            Some('r') => out.push(b'\r'),
            Some('t') => out.push(b'\t'),
            Some('0') => out.push(0),
            Some('\\') => out.push(b'\\'),
            Some('"') => out.push(b'"'),
            Some('\'') => out.push(b'\''),
            Some('x') => {
                let h1 = it.next();
                let h2 = it.next();
                match (h1, h2) {
                    (Some(a), Some(b)) => {
                        let hex_pair = [a as u8, b as u8];
                        let hex_str = std::str::from_utf8(&hex_pair).unwrap_or("");
                        if let Ok(v) = u8::from_str_radix(hex_str, 16) {
                            out.push(v);
                        } else {
                            out.push(b'\\');
                            out.push(b'x');
                            if let Some(c) = h1 {
                                let mut buf = [0u8; 4];
                                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                            }
                            if let Some(c) = h2 {
                                let mut buf = [0u8; 4];
                                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                            }
                        }
                    }
                    _ => {
                        out.push(b'\\');
                        out.push(b'x');
                    }
                }
            }
            Some(other) => {
                // Unknown escape — pass through both chars verbatim.
                out.push(b'\\');
                let mut buf = [0u8; 4];
                out.extend_from_slice(other.encode_utf8(&mut buf).as_bytes());
            }
            None => out.push(b'\\'),
        }
    }
    out
}

#[no_mangle]
pub extern "Rust" fn bytes_from_string(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let s = str_arg(&args, 0);
    ok_bytes(ctx.bytes_new(decode_escapes(&s)))
}

#[no_mangle]
pub extern "Rust" fn bytes_len(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match bytes_arg(&args, 0) { Some(h) => h, None => return ok_int(0) };
    ok_int(ctx.bytes_len(h) as isize)
}

#[no_mangle]
pub extern "Rust" fn bytes_pos(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match bytes_arg(&args, 0) { Some(h) => h, None => return ok_int(0) };
    ok_int(ctx.bytes_pos(h) as isize)
}

#[no_mangle]
pub extern "Rust" fn bytes_set_pos(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    if let Some(h) = bytes_arg(&args, 0) {
        let p = int_arg(&args, 1).max(0) as usize;
        ctx.bytes_set_pos(h, p);
    }
    ok(())
}

// Returns byte value as int (0..255). The language doesn't yet support
// `int as byte` coercion, so the user surface for byte ops is int.
#[no_mangle]
pub extern "Rust" fn bytes_get(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match bytes_arg(&args, 0) { Some(h) => h, None => return ok_int(0) };
    let i = int_arg(&args, 1).max(0) as usize;
    ok_int(ctx.bytes_get(h, i).unwrap_or(0) as isize)
}

#[no_mangle]
pub extern "Rust" fn bytes_set(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    if let Some(h) = bytes_arg(&args, 0) {
        let i = int_arg(&args, 1).max(0) as usize;
        let v = byte_arg(&args, 2);
        ctx.bytes_set(h, i, v);
    }
    ok(())
}

#[no_mangle]
pub extern "Rust" fn bytes_slice(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match bytes_arg(&args, 0) {
        Some(h) => h,
        None => return ok_bytes(ctx.bytes_new(Vec::new())),
    };
    let start = int_arg(&args, 1).max(0) as usize;
    let end = int_arg(&args, 2).max(0) as usize;
    let view = ctx.bytes_view(h).unwrap_or(&[]);
    let start_c = start.min(view.len());
    let end_c = end.min(view.len()).max(start_c);
    let copy = view[start_c..end_c].to_vec();
    ok_bytes(ctx.bytes_new(copy))
}

#[no_mangle]
pub extern "Rust" fn bytes_append(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let dst = match bytes_arg(&args, 0) { Some(h) => h, None => return ok(()) };
    let src = match bytes_arg(&args, 1) { Some(h) => h, None => return ok(()) };
    let src_data = ctx.bytes_clone(src).unwrap_or_default();
    ctx.bytes_append(dst, &src_data);
    ok(())
}

#[no_mangle]
pub extern "Rust" fn bytes_to_string(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match bytes_arg(&args, 0) { Some(h) => h, None => return ok(String::new()) };
    let view = ctx.bytes_view(h).unwrap_or(&[]);
    ok(String::from_utf8_lossy(view).into_owned())
}

// Treats the byte buffer as a little-endian u32 array. Useful for pixel
// buffers (minifb expects &[u32]) and any binary protocol with 32-bit words.
// Pixel/word index is 1/4 of the byte offset.

#[no_mangle]
pub extern "Rust" fn bytes_set_u32(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match bytes_arg(&args, 0) { Some(h) => h, None => return ok(()) };
    let idx = int_arg(&args, 1).max(0) as usize;
    let value = int_arg(&args, 2) as u32;
    if let Some(slice) = ctx.bytes_mut_slice(h, idx.wrapping_mul(4), 4) {
        slice.copy_from_slice(&value.to_le_bytes());
    }
    ok(())
}

#[no_mangle]
pub extern "Rust" fn bytes_get_u32(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match bytes_arg(&args, 0) { Some(h) => h, None => return ok_int(0) };
    let idx = int_arg(&args, 1).max(0) as usize;
    let view = ctx.bytes_view(h).unwrap_or(&[]);
    let off = idx.wrapping_mul(4);
    if off + 4 > view.len() {
        return ok_int(0);
    }
    let v = u32::from_le_bytes([view[off], view[off + 1], view[off + 2], view[off + 3]]);
    ok_int(v as isize)
}

#[no_mangle]
pub extern "Rust" fn bytes_fill_u32(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match bytes_arg(&args, 0) { Some(h) => h, None => return ok(()) };
    let start = int_arg(&args, 1).max(0) as usize;
    let end = int_arg(&args, 2).max(0) as usize;
    let value = int_arg(&args, 3) as u32;
    if end <= start {
        return ok(());
    }
    let byte_start = start.wrapping_mul(4);
    let byte_len = end.wrapping_sub(start).wrapping_mul(4);
    if let Some(slice) = ctx.bytes_mut_slice(h, byte_start, byte_len) {
        let bytes = value.to_le_bytes();
        for chunk in slice.chunks_exact_mut(4) {
            chunk.copy_from_slice(&bytes);
        }
    }
    ok(())
}

// ── net.ei — TCP ─────────────────────────────────────────────────────────────

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::time::Duration;

fn parse_addr(host: &str, port: isize) -> Option<String> {
    if port < 0 || port > 65535 {
        return None;
    }
    if host.contains(':') {
        // IPv6 literal — must be bracketed in the address string.
        Some(format!("[{}]:{}", host, port))
    } else {
        Some(format!("{}:{}", host, port))
    }
}

fn opt_duration(ms: isize) -> Option<Duration> {
    if ms <= 0 {
        None
    } else {
        Some(Duration::from_millis(ms as u64))
    }
}

#[no_mangle]
pub extern "Rust" fn tcp_connect(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let host = str_arg(&args, 0);
    let port = int_arg(&args, 1);
    let timeout_ms = int_arg(&args, 2);
    let addr = match parse_addr(&host, port) {
        Some(a) => a,
        None => return ok_null(),
    };
    let stream = if let Some(dur) = opt_duration(timeout_ms) {
        // connect_timeout requires a SocketAddr, not a string — resolve first.
        let resolved: Option<SocketAddr> = match addr.parse() {
            Ok(s) => Some(s),
            Err(_) => addr.to_socket_addrs_first(),
        };
        match resolved {
            Some(sa) => TcpStream::connect_timeout(&sa, dur).ok(),
            None => TcpStream::connect(&addr).ok(),
        }
    } else {
        TcpStream::connect(&addr).ok()
    };
    match stream {
        Some(s) => ok_handle(ctx.handle_new(Box::new(s))),
        None => ok_null(),
    }
}

trait ToSocketAddrFirst {
    fn to_socket_addrs_first(&self) -> Option<SocketAddr>;
}

impl ToSocketAddrFirst for String {
    fn to_socket_addrs_first(&self) -> Option<SocketAddr> {
        use std::net::ToSocketAddrs;
        self.as_str().to_socket_addrs().ok().and_then(|mut it| it.next())
    }
}

#[no_mangle]
pub extern "Rust" fn tcp_listen(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let host = str_arg(&args, 0);
    let port = int_arg(&args, 1);
    let addr = match parse_addr(&host, port) {
        Some(a) => a,
        None => return ok_null(),
    };
    match TcpListener::bind(&addr) {
        Ok(l) => ok_handle(ctx.handle_new(Box::new(l))),
        Err(_) => ok_null(),
    }
}

#[no_mangle]
pub extern "Rust" fn tcp_accept(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match handle_arg(&args, 0) { Some(h) => h, None => return ok_null() };
    let stream_opt = ctx
        .handle_view(h)
        .and_then(|a| a.downcast_ref::<TcpListener>())
        .and_then(|l| l.accept().ok())
        .map(|(s, _)| s);
    match stream_opt {
        Some(s) => ok_handle(ctx.handle_new(Box::new(s))),
        None => ok_null(),
    }
}

#[no_mangle]
pub extern "Rust" fn tcp_read(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match handle_arg(&args, 0) { Some(h) => h, None => return ok_int(-1) };
    let buf_h = match bytes_arg(&args, 1) { Some(h) => h, None => return ok_int(-1) };
    let offset = int_arg(&args, 2).max(0) as usize;
    let len = int_arg(&args, 3).max(0) as usize;

    // Temporary buffer because we can't hold a &mut TcpStream and &mut [u8]
    // from the same heap simultaneously.
    let mut tmp = vec![0u8; len];
    let n = match ctx
        .handle_view_mut(h)
        .and_then(|a| a.downcast_mut::<TcpStream>())
    {
        Some(stream) => match stream.read(&mut tmp) {
            Ok(n) => n,
            Err(_) => return ok_int(-1),
        },
        None => return ok_int(-1),
    };

    if let Some(slice) = ctx.bytes_mut_slice(buf_h, offset, n) {
        slice.copy_from_slice(&tmp[..n]);
    }
    ok_int(n as isize)
}

#[no_mangle]
pub extern "Rust" fn tcp_write(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match handle_arg(&args, 0) { Some(h) => h, None => return ok_int(-1) };
    let buf_h = match bytes_arg(&args, 1) { Some(h) => h, None => return ok_int(-1) };
    let offset = int_arg(&args, 2).max(0) as usize;
    let len = int_arg(&args, 3).max(0) as usize;

    // Snapshot the relevant slice; we can't hold both borrows.
    let data = match ctx.bytes_view(buf_h) {
        Some(v) => {
            let end = (offset + len).min(v.len());
            if offset > end { return ok_int(0); }
            v[offset..end].to_vec()
        }
        None => return ok_int(-1),
    };

    match ctx
        .handle_view_mut(h)
        .and_then(|a| a.downcast_mut::<TcpStream>())
    {
        Some(stream) => match stream.write(&data) {
            Ok(n) => ok_int(n as isize),
            Err(_) => ok_int(-1),
        },
        None => ok_int(-1),
    }
}

#[no_mangle]
pub extern "Rust" fn tcp_set_read_timeout(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    if let Some(h) = handle_arg(&args, 0) {
        let ms = int_arg(&args, 1);
        if let Some(stream) = ctx
            .handle_view_mut(h)
            .and_then(|a| a.downcast_mut::<TcpStream>())
        {
            let _ = stream.set_read_timeout(opt_duration(ms));
        }
    }
    ok(())
}

#[no_mangle]
pub extern "Rust" fn tcp_set_write_timeout(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    if let Some(h) = handle_arg(&args, 0) {
        let ms = int_arg(&args, 1);
        if let Some(stream) = ctx
            .handle_view_mut(h)
            .and_then(|a| a.downcast_mut::<TcpStream>())
        {
            let _ = stream.set_write_timeout(opt_duration(ms));
        }
    }
    ok(())
}

#[no_mangle]
pub extern "Rust" fn tcp_peer_addr(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match handle_arg(&args, 0) { Some(h) => h, None => return ok(String::new()) };
    let addr = ctx
        .handle_view(h)
        .and_then(|a| a.downcast_ref::<TcpStream>())
        .and_then(|s| s.peer_addr().ok())
        .map(|a| a.to_string())
        .unwrap_or_default();
    ok(addr)
}

#[no_mangle]
pub extern "Rust" fn tcp_local_addr(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match handle_arg(&args, 0) { Some(h) => h, None => return ok(String::new()) };
    // Works for either TcpStream or TcpListener — try each.
    let addr = ctx.handle_view(h).and_then(|a| {
        if let Some(s) = a.downcast_ref::<TcpStream>() {
            s.local_addr().ok()
        } else if let Some(l) = a.downcast_ref::<TcpListener>() {
            l.local_addr().ok()
        } else {
            None
        }
    });
    ok(addr.map(|a| a.to_string()).unwrap_or_default())
}

#[no_mangle]
pub extern "Rust" fn tcp_close(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    if let Some(h) = handle_arg(&args, 0) {
        ctx.handle_drop(h);
    }
    ok(())
}

// ── net.ei — UDP ─────────────────────────────────────────────────────────────
//
// recv_from yields two values (length + source address). Since multi-return
// isn't ergonomic yet, the socket carries last-recv state retrievable via
// udp_last_host / udp_last_port after each udp_recv_into call.

struct UdpHandle {
    sock: UdpSocket,
    last_host: String,
    last_port: u16,
}

#[no_mangle]
pub extern "Rust" fn udp_bind(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let host = str_arg(&args, 0);
    let port = int_arg(&args, 1);
    let addr = match parse_addr(&host, port) {
        Some(a) => a,
        None => return ok_null(),
    };
    match UdpSocket::bind(&addr) {
        Ok(s) => ok_handle(ctx.handle_new(Box::new(UdpHandle {
            sock: s,
            last_host: String::new(),
            last_port: 0,
        }))),
        Err(_) => ok_null(),
    }
}

#[no_mangle]
pub extern "Rust" fn udp_send_to(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match handle_arg(&args, 0) { Some(h) => h, None => return ok_int(-1) };
    let buf_h = match bytes_arg(&args, 1) { Some(h) => h, None => return ok_int(-1) };
    let offset = int_arg(&args, 2).max(0) as usize;
    let len = int_arg(&args, 3).max(0) as usize;
    let host = str_arg(&args, 4);
    let port = int_arg(&args, 5);

    let data = match ctx.bytes_view(buf_h) {
        Some(v) => {
            let end = (offset + len).min(v.len());
            if offset > end { return ok_int(0); }
            v[offset..end].to_vec()
        }
        None => return ok_int(-1),
    };
    let addr = match parse_addr(&host, port) {
        Some(a) => a,
        None => return ok_int(-1),
    };
    match ctx
        .handle_view_mut(h)
        .and_then(|a| a.downcast_mut::<UdpHandle>())
    {
        Some(udp) => match udp.sock.send_to(&data, &addr) {
            Ok(n) => ok_int(n as isize),
            Err(_) => ok_int(-1),
        },
        None => ok_int(-1),
    }
}

#[no_mangle]
pub extern "Rust" fn udp_recv_into(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match handle_arg(&args, 0) { Some(h) => h, None => return ok_int(-1) };
    let buf_h = match bytes_arg(&args, 1) { Some(h) => h, None => return ok_int(-1) };
    let offset = int_arg(&args, 2).max(0) as usize;
    let len = int_arg(&args, 3).max(0) as usize;

    let mut tmp = vec![0u8; len];
    let (n, from_host, from_port) = match ctx
        .handle_view_mut(h)
        .and_then(|a| a.downcast_mut::<UdpHandle>())
    {
        Some(udp) => match udp.sock.recv_from(&mut tmp) {
            Ok((n, src)) => {
                let host = src.ip().to_string();
                let port = src.port();
                udp.last_host = host.clone();
                udp.last_port = port;
                (n, host, port)
            }
            Err(_) => return ok_int(-1),
        },
        None => return ok_int(-1),
    };
    let _ = (from_host, from_port); // already stored on the handle

    if let Some(slice) = ctx.bytes_mut_slice(buf_h, offset, n) {
        slice.copy_from_slice(&tmp[..n]);
    }
    ok_int(n as isize)
}

#[no_mangle]
pub extern "Rust" fn udp_last_host(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match handle_arg(&args, 0) { Some(h) => h, None => return ok(String::new()) };
    let host = ctx
        .handle_view(h)
        .and_then(|a| a.downcast_ref::<UdpHandle>())
        .map(|u| u.last_host.clone())
        .unwrap_or_default();
    ok(host)
}

#[no_mangle]
pub extern "Rust" fn udp_last_port(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match handle_arg(&args, 0) { Some(h) => h, None => return ok_int(0) };
    let port = ctx
        .handle_view(h)
        .and_then(|a| a.downcast_ref::<UdpHandle>())
        .map(|u| u.last_port as isize)
        .unwrap_or(0);
    ok_int(port)
}

#[no_mangle]
pub extern "Rust" fn udp_local_addr(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let h = match handle_arg(&args, 0) { Some(h) => h, None => return ok(String::new()) };
    let addr = ctx
        .handle_view(h)
        .and_then(|a| a.downcast_ref::<UdpHandle>())
        .and_then(|u| u.sock.local_addr().ok())
        .map(|a| a.to_string())
        .unwrap_or_default();
    ok(addr)
}

#[no_mangle]
pub extern "Rust" fn udp_set_read_timeout(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    if let Some(h) = handle_arg(&args, 0) {
        let ms = int_arg(&args, 1);
        if let Some(udp) = ctx
            .handle_view_mut(h)
            .and_then(|a| a.downcast_mut::<UdpHandle>())
        {
            let _ = udp.sock.set_read_timeout(opt_duration(ms));
        }
    }
    ok(())
}

#[no_mangle]
pub extern "Rust" fn udp_close(ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    if let Some(h) = handle_arg(&args, 0) {
        ctx.handle_drop(h);
    }
    ok(())
}

// ── time.ei ──────────────────────────────────────────────────────────────────

fn ms_to_utc(ms: isize) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::TimeZone;
    chrono::Utc.timestamp_millis_opt(ms as i64).single()
}

#[no_mangle]
pub extern "Rust" fn time_now_ms(_ctx: &mut dyn EllieCtx, _args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use std::time::SystemTime;
    let ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as isize;
    ok_int(ms)
}

#[no_mangle]
pub extern "Rust" fn time_now_s(_ctx: &mut dyn EllieCtx, _args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use std::time::SystemTime;
    let s = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as isize;
    ok_int(s)
}

#[no_mangle]
pub extern "Rust" fn time_now_iso(_ctx: &mut dyn EllieCtx, _args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use chrono::SecondsFormat;
    let now = chrono::Utc::now();
    ok(now.to_rfc3339_opts(SecondsFormat::Millis, true))
}

#[no_mangle]
pub extern "Rust" fn time_format(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let ms = int_arg(&args, 0);
    let fmt = str_arg(&args, 1);
    let s = ms_to_utc(ms)
        .map(|dt| dt.format(&fmt).to_string())
        .unwrap_or_default();
    ok(s)
}

#[no_mangle]
pub extern "Rust" fn time_parse(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let s = str_arg(&args, 0);
    let fmt = str_arg(&args, 1);
    let ms = chrono::NaiveDateTime::parse_from_str(&s, &fmt)
        .map(|naive| naive.and_utc().timestamp_millis() as isize)
        .unwrap_or(-1);
    ok_int(ms)
}

#[no_mangle]
pub extern "Rust" fn time_year(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use chrono::Datelike;
    let ms = int_arg(&args, 0);
    ok_int(ms_to_utc(ms).map(|dt| dt.year() as isize).unwrap_or(0))
}

#[no_mangle]
pub extern "Rust" fn time_month(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use chrono::Datelike;
    let ms = int_arg(&args, 0);
    ok_int(ms_to_utc(ms).map(|dt| dt.month() as isize).unwrap_or(0))
}

#[no_mangle]
pub extern "Rust" fn time_day(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use chrono::Datelike;
    let ms = int_arg(&args, 0);
    ok_int(ms_to_utc(ms).map(|dt| dt.day() as isize).unwrap_or(0))
}

#[no_mangle]
pub extern "Rust" fn time_hour(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use chrono::Timelike;
    let ms = int_arg(&args, 0);
    ok_int(ms_to_utc(ms).map(|dt| dt.hour() as isize).unwrap_or(0))
}

#[no_mangle]
pub extern "Rust" fn time_minute(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use chrono::Timelike;
    let ms = int_arg(&args, 0);
    ok_int(ms_to_utc(ms).map(|dt| dt.minute() as isize).unwrap_or(0))
}

#[no_mangle]
pub extern "Rust" fn time_second(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use chrono::Timelike;
    let ms = int_arg(&args, 0);
    ok_int(ms_to_utc(ms).map(|dt| dt.second() as isize).unwrap_or(0))
}

// ── encoding.ei ──────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn base64_encode(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use base64::Engine;
    let s = str_arg(&args, 0);
    ok(base64::engine::general_purpose::STANDARD.encode(s.as_bytes()))
}

#[no_mangle]
pub extern "Rust" fn base64_decode(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use base64::Engine;
    let s = str_arg(&args, 0);
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(s.as_bytes())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_default();
    ok(decoded)
}

#[no_mangle]
pub extern "Rust" fn hex_encode(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let s = str_arg(&args, 0);
    ok(hex::encode(s.as_bytes()))
}

#[no_mangle]
pub extern "Rust" fn hex_decode(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let s = str_arg(&args, 0);
    let decoded = hex::decode(&s)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_default();
    ok(decoded)
}

// ── crypto.ei ────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn crypto_sha256_hex(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use sha2::{Digest, Sha256};
    let s = str_arg(&args, 0);
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    ok(hex::encode(hasher.finalize()))
}

#[no_mangle]
pub extern "Rust" fn crypto_sha512_hex(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use sha2::{Digest, Sha512};
    let s = str_arg(&args, 0);
    let mut hasher = Sha512::new();
    hasher.update(s.as_bytes());
    ok(hex::encode(hasher.finalize()))
}

fn next_random_u64() -> u64 {
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0) };
    }
    STATE.with(|cell| {
        let mut s = cell.get();
        if s == 0 {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0xdeadbeef);
            s = nanos
                .wrapping_mul(0x9E3779B97F4A7C15)
                .wrapping_add(std::process::id() as u64);
            if s == 0 {
                s = 0x12345678;
            }
        }
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        cell.set(s);
        s
    })
}

#[no_mangle]
pub extern "Rust" fn random_int(_ctx: &mut dyn EllieCtx, args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let min = int_arg(&args, 0);
    let max = int_arg(&args, 1);
    if max <= min {
        return ok_int(min);
    }
    let span = (max - min) as u64;
    let r = (next_random_u64() % span) as isize + min;
    ok_int(r)
}

// ── Module registration ───────────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn load_module() -> EllieModule {
    EllieModule {
        abi_version: ELLIE_RUST_ABI_VERSION,
        name: "ellieCore",
        version: env!("CARGO_PKG_VERSION"),
        functions: vec![
            // platform.ei
            EllieFunction { name: "get_arch",            on_call: get_arch            },
            EllieFunction { name: "get_os",              on_call: get_os              },
            EllieFunction { name: "get_eol",             on_call: get_eol             },
            EllieFunction { name: "print",               on_call: print               },
            EllieFunction { name: "println",             on_call: println             },
            EllieFunction { name: "panic",               on_call: panic               },
            EllieFunction { name: "timestamp",           on_call: timestamp           },
            EllieFunction { name: "sleep_ms",            on_call: sleep_ms            },

            // build.ei — process / shell
            EllieFunction { name: "command_exists",      on_call: command_exists      },
            EllieFunction { name: "run_command",         on_call: run_command         },
            EllieFunction { name: "run_command_silent",  on_call: run_command_silent  },
            EllieFunction { name: "run_command_output",  on_call: run_command_output  },

            // build.ei — env / cwd
            EllieFunction { name: "get_env",             on_call: get_env             },
            EllieFunction { name: "set_env",             on_call: set_env             },
            EllieFunction { name: "get_cwd",             on_call: get_cwd             },
            EllieFunction { name: "set_cwd",             on_call: set_cwd             },

            // build.ei — current process
            EllieFunction { name: "process_args",        on_call: process_args        },
            EllieFunction { name: "process_pid",         on_call: process_pid         },
            EllieFunction { name: "process_exit",        on_call: process_exit        },

            // fs.ei
            EllieFunction { name: "fs_read_file",        on_call: fs_read_file        },
            EllieFunction { name: "fs_write_file",       on_call: fs_write_file       },
            EllieFunction { name: "fs_append_file",      on_call: fs_append_file      },
            EllieFunction { name: "fs_exists",           on_call: fs_exists           },
            EllieFunction { name: "fs_is_file",          on_call: fs_is_file          },
            EllieFunction { name: "fs_is_dir",           on_call: fs_is_dir           },
            EllieFunction { name: "fs_remove_file",      on_call: fs_remove_file      },
            EllieFunction { name: "fs_remove_dir",       on_call: fs_remove_dir       },
            EllieFunction { name: "fs_remove_dir_all",   on_call: fs_remove_dir_all   },
            EllieFunction { name: "fs_rename",           on_call: fs_rename           },
            EllieFunction { name: "fs_create_dir",       on_call: fs_create_dir       },
            EllieFunction { name: "fs_create_dir_all",   on_call: fs_create_dir_all   },
            EllieFunction { name: "fs_read_dir",         on_call: fs_read_dir         },
            EllieFunction { name: "fs_size",             on_call: fs_size             },
            EllieFunction { name: "fs_modified_ms",      on_call: fs_modified_ms      },
            EllieFunction { name: "fs_read_bytes",       on_call: fs_read_bytes       },
            EllieFunction { name: "fs_write_bytes",      on_call: fs_write_bytes      },
            EllieFunction { name: "fs_read_into",        on_call: fs_read_into        },

            // bytes.ei
            EllieFunction { name: "bytes_alloc",         on_call: bytes_alloc         },
            EllieFunction { name: "bytes_from_string",   on_call: bytes_from_string   },
            EllieFunction { name: "bytes_len",           on_call: bytes_len           },
            EllieFunction { name: "bytes_pos",           on_call: bytes_pos           },
            EllieFunction { name: "bytes_set_pos",       on_call: bytes_set_pos       },
            EllieFunction { name: "bytes_get",           on_call: bytes_get           },
            EllieFunction { name: "bytes_set",           on_call: bytes_set           },
            EllieFunction { name: "bytes_slice",         on_call: bytes_slice         },
            EllieFunction { name: "bytes_append",        on_call: bytes_append        },
            EllieFunction { name: "bytes_to_string",     on_call: bytes_to_string     },
            EllieFunction { name: "bytes_set_u32",       on_call: bytes_set_u32       },
            EllieFunction { name: "bytes_get_u32",       on_call: bytes_get_u32       },
            EllieFunction { name: "bytes_fill_u32",      on_call: bytes_fill_u32      },

            // net.ei — TCP
            EllieFunction { name: "tcp_connect",            on_call: tcp_connect            },
            EllieFunction { name: "tcp_listen",             on_call: tcp_listen             },
            EllieFunction { name: "tcp_accept",             on_call: tcp_accept             },
            EllieFunction { name: "tcp_read",               on_call: tcp_read               },
            EllieFunction { name: "tcp_write",              on_call: tcp_write              },
            EllieFunction { name: "tcp_set_read_timeout",   on_call: tcp_set_read_timeout   },
            EllieFunction { name: "tcp_set_write_timeout",  on_call: tcp_set_write_timeout  },
            EllieFunction { name: "tcp_peer_addr",          on_call: tcp_peer_addr          },
            EllieFunction { name: "tcp_local_addr",         on_call: tcp_local_addr         },
            EllieFunction { name: "tcp_close",              on_call: tcp_close              },

            // net.ei — UDP
            EllieFunction { name: "udp_bind",               on_call: udp_bind               },
            EllieFunction { name: "udp_send_to",            on_call: udp_send_to            },
            EllieFunction { name: "udp_recv_into",          on_call: udp_recv_into          },
            EllieFunction { name: "udp_last_host",          on_call: udp_last_host          },
            EllieFunction { name: "udp_last_port",          on_call: udp_last_port          },
            EllieFunction { name: "udp_local_addr",         on_call: udp_local_addr         },
            EllieFunction { name: "udp_set_read_timeout",   on_call: udp_set_read_timeout   },
            EllieFunction { name: "udp_close",              on_call: udp_close              },

            // time.ei
            EllieFunction { name: "time_now_ms",         on_call: time_now_ms         },
            EllieFunction { name: "time_now_s",          on_call: time_now_s          },
            EllieFunction { name: "time_now_iso",        on_call: time_now_iso        },
            EllieFunction { name: "time_format",         on_call: time_format         },
            EllieFunction { name: "time_parse",          on_call: time_parse          },
            EllieFunction { name: "time_year",           on_call: time_year           },
            EllieFunction { name: "time_month",          on_call: time_month          },
            EllieFunction { name: "time_day",            on_call: time_day            },
            EllieFunction { name: "time_hour",           on_call: time_hour           },
            EllieFunction { name: "time_minute",         on_call: time_minute         },
            EllieFunction { name: "time_second",         on_call: time_second         },

            // encoding.ei
            EllieFunction { name: "base64_encode",       on_call: base64_encode       },
            EllieFunction { name: "base64_decode",       on_call: base64_decode       },
            EllieFunction { name: "hex_encode",          on_call: hex_encode          },
            EllieFunction { name: "hex_decode",          on_call: hex_decode          },

            // crypto.ei
            EllieFunction { name: "crypto_sha256_hex",   on_call: crypto_sha256_hex   },
            EllieFunction { name: "crypto_sha512_hex",   on_call: crypto_sha512_hex   },
            EllieFunction { name: "random_int",          on_call: random_int          },
        ],
    }
}
