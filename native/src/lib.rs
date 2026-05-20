use ellie_native_bridge::rust::{
    EllieData, EllieFunction, EllieModule, FunctionAnswer, FunctionCallParameter,
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

fn ok(data: impl Into<EllieData>) -> FunctionAnswer {
    FunctionAnswer::Ok(data.into())
}

// ── platform.ei ───────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn get_arch(_args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    ok(std::env::consts::ARCH)
}

#[no_mangle]
pub extern "Rust" fn get_os(_args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    ok(std::env::consts::OS)
}

#[no_mangle]
pub extern "Rust" fn get_eol(_args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    #[cfg(target_os = "windows")]
    let eol = "\r\n";
    #[cfg(not(target_os = "windows"))]
    let eol = "\n";
    ok(eol)
}

#[no_mangle]
pub extern "Rust" fn print(args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let s = str_arg(&args, 0);
    std::print!("{}", s);
    ok(())
}

#[no_mangle]
pub extern "Rust" fn println(args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let s = str_arg(&args, 0);
    std::println!("{}", s);
    ok(())
}

#[no_mangle]
pub extern "Rust" fn panic(args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let msg = str_arg(&args, 0);
    eprintln!("panic: {}", msg);
    std::process::exit(1);
}

#[no_mangle]
pub extern "Rust" fn timestamp(_args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    use std::time::SystemTime;
    let ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as isize;
    ok(ms)
}

#[no_mangle]
pub extern "Rust" fn sleep_ms(args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let ms = int_arg(&args, 0);
    std::thread::sleep(std::time::Duration::from_millis(ms.max(0) as u64));
    ok(())
}

// ── build.ei — process / shell ────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn command_exists(args: Vec<FunctionCallParameter>) -> FunctionAnswer {
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
    ok(found)
}

#[no_mangle]
pub extern "Rust" fn run_command(args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let cmd = str_arg(&args, 0);
    let cmd_args = string_array_arg(&args, 1);
    let code = std::process::Command::new(&cmd)
        .args(&cmd_args)
        .status()
        .map(|s| s.code().unwrap_or(-1))
        .unwrap_or(-1);
    ok(code as isize)
}

#[no_mangle]
pub extern "Rust" fn run_command_silent(args: Vec<FunctionCallParameter>) -> FunctionAnswer {
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
    ok(code as isize)
}

#[no_mangle]
pub extern "Rust" fn run_command_output(args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let cmd = str_arg(&args, 0);
    let cmd_args = string_array_arg(&args, 1);
    let out = std::process::Command::new(&cmd)
        .args(&cmd_args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    ok(out)
}

// ── build.ei — environment ────────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn get_env(args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let name = str_arg(&args, 0);
    ok(std::env::var(&name).unwrap_or_default())
}

#[no_mangle]
pub extern "Rust" fn set_env(args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let name = str_arg(&args, 0);
    let value = str_arg(&args, 1);
    unsafe { std::env::set_var(&name, &value) };
    ok(())
}

#[no_mangle]
pub extern "Rust" fn get_cwd(_args: Vec<FunctionCallParameter>) -> FunctionAnswer {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    ok(cwd)
}

// ── Array helper (EllieData::Array of strings) ────────────────────────────────

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

// ── Module registration ───────────────────────────────────────────────────────

#[no_mangle]
pub extern "Rust" fn load_module() -> EllieModule {
    EllieModule {
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
            // build.ei — process
            EllieFunction { name: "command_exists",      on_call: command_exists      },
            EllieFunction { name: "run_command",         on_call: run_command         },
            EllieFunction { name: "run_command_silent",  on_call: run_command_silent  },
            EllieFunction { name: "run_command_output",  on_call: run_command_output  },
            // build.ei — environment
            EllieFunction { name: "get_env",             on_call: get_env             },
            EllieFunction { name: "set_env",             on_call: set_env             },
            EllieFunction { name: "get_cwd",             on_call: get_cwd             },
        ],
    }
}
