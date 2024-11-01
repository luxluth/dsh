use dsh::{
    cmd,
    internals::{self, get_internal_functions_map},
};

use nix::sys::signal::{self, SigHandler, Signal};
use std::{
    env,
    io::{self, prelude::*},
    os::fd::AsRawFd,
    sync::atomic::{AtomicBool, AtomicI32, Ordering},
};

static NEED_STOP: AtomicBool = AtomicBool::new(false);
static STDIN_FD: AtomicI32 = AtomicI32::new(0);

struct Shell {
    internals: Option<dsh::internals::InternalFuncMap>,
}

extern "C" fn handle_sighup(signal: libc::c_int) {
    let signal = Signal::try_from(signal).unwrap();
    NEED_STOP.store(signal == Signal::SIGHUP, Ordering::Relaxed);
    std::process::exit(0);
}

impl Shell {
    const fn new() -> Self {
        Self { internals: None }
    }

    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.internals = Some(get_internal_functions_map());

        let stdin = io::stdin();
        STDIN_FD.store(stdin.as_raw_fd(), Ordering::Relaxed);
        let mut stdout = io::stdout();
        let hostname_file = std::path::Path::new("/etc/hostname");
        if hostname_file.exists() {
            let mut f = std::fs::File::open(hostname_file).unwrap();
            let mut content = String::new();
            let _ = f.read_to_string(&mut content);

            if !content.is_empty() {
                env::set_var("hostname", content.trim());
            }
        }

        let mut error_code = 0;

        loop {
            let prompt = format!(
                "{}@{} [{}] ",
                env::var("USERNAME").unwrap_or("".to_string()),
                env::var("hostname").unwrap_or("".to_string()),
                error_code,
            );
            let mut buffer = String::new();
            // Check if the shell should stop.
            if NEED_STOP.load(Ordering::Relaxed) {
                break;
            }

            stdout.write_all(prompt.as_bytes())?;
            stdout.flush()?;

            if let Ok(read) = stdin.read_line(&mut buffer) {
                if read > 0 {
                    let line = buffer.trim();
                    if !line.is_empty() {
                        if let Ok(cmd) = cmd::Cmd::new(line) {
                            match cmd.name.as_str() {
                                "exit" => {
                                    break;
                                }
                                "clear" | "cd" => {
                                    if let Some(func) =
                                        self.internals.as_ref().unwrap().get(&cmd.name)
                                    {
                                        match func(cmd) {
                                            Ok(code) => {
                                                error_code = code.code().unwrap_or(-1);
                                                std::env::set_var(
                                                    "STATUS",
                                                    code.code().unwrap_or(-1).to_string(),
                                                );
                                            }
                                            Err(e) => {
                                                eprintln!("{e}");
                                                match e {
                                                    internals::CommandError::IOError(_) => {
                                                        error_code = 1;
                                                        std::env::set_var("STATUS", 1.to_string());
                                                    }
                                                    internals::CommandError::Custom {
                                                        status,
                                                        ..
                                                    }
                                                    | internals::CommandError::ChildSpawnError(
                                                        _,
                                                        _,
                                                        status,
                                                    ) => {
                                                        error_code = status;
                                                        std::env::set_var(
                                                            "STATUS",
                                                            status.to_string(),
                                                        );
                                                    }
                                                }
                                            }
                                        };
                                    }
                                }
                                _ => match internals::run(cmd) {
                                    Ok(code) => {
                                        error_code = code.code().unwrap_or(127);
                                        std::env::set_var(
                                            "STATUS",
                                            code.code().unwrap_or(127).to_string(),
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!("{e}");
                                        match e {
                                            internals::CommandError::IOError(_) => {
                                                error_code = 1;
                                                std::env::set_var("STATUS", 1.to_string());
                                            }
                                            internals::CommandError::Custom { status, .. }
                                            | internals::CommandError::ChildSpawnError(
                                                _,
                                                _,
                                                status,
                                            ) => {
                                                error_code = status;
                                                std::env::set_var("STATUS", status.to_string());
                                            }
                                        }
                                    }
                                },
                            }
                        }
                    }
                } else {
                    // EOF
                    break;
                }
            } else {
                // Stdin error
                break;
            }
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = SigHandler::Handler(handle_sighup);
    unsafe { signal::signal(Signal::SIGHUP, handler) }.unwrap();
    unsafe { signal::signal(Signal::SIGINT, handler) }.unwrap();
    let mut shell = Shell::new();
    shell.run()?;

    Ok(())
}
