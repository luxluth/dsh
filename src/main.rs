use dsh::{
    cmd,
    internals::{self, get_internal_functions_map},
};
use std::{
    env,
    io::{self, prelude::*},
    sync::{Arc, Mutex},
};

struct Shell {
    should_stop: Arc<Mutex<bool>>,
    internals: dsh::internals::InternalFuncMap,
}

impl Shell {
    fn new() -> Self {
        Self {
            should_stop: Arc::new(Mutex::new(false)),
            internals: get_internal_functions_map(),
        }
    }

    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = io::stdin();
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
            if *self.should_stop.lock().unwrap() {
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
                                    if let Some(func) = self.internals.get(&cmd.name) {
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
    let mut shell = Shell::new();
    shell.run()?;

    Ok(())
}
