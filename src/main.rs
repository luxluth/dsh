use internals::get_internal_functions_map;
use std::{
    io::{self, prelude::*},
    sync::{Arc, Mutex},
};

mod cmd;
mod internals;

const PROMPT: &[u8] = b"# ";

struct Shell {
    should_stop: Arc<Mutex<bool>>,
    internals: internals::InternalFuncMap,
}

impl Shell {
    fn new() -> Self {
        Self {
            should_stop: Arc::new(Mutex::new(false)),
            internals: get_internal_functions_map(),
        }
    }

    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        let stdin = io::stdin();

        loop {
            let mut buffer = String::new();
            // Check if the shell should stop.
            if *self.should_stop.lock().unwrap() {
                break;
            }

            stdout.write_all(PROMPT)?;
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
                                "clear" => {
                                    if let Some(func) = self.internals.get(&cmd.name) {
                                        match func(cmd) {
                                            Ok(code) => {
                                                std::env::set_var(
                                                    "STATUS",
                                                    code.code().unwrap_or(127).to_string(),
                                                );
                                            }
                                            Err(e) => {
                                                eprintln!("{e}");
                                            }
                                        };
                                    }
                                }
                                _ => match internals::run(cmd) {
                                    Ok(code) => {
                                        std::env::set_var(
                                            "STATUS",
                                            code.code().unwrap_or(127).to_string(),
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!("{e}");
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
