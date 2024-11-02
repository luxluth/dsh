use std::{
    collections::HashMap,
    env,
    io::{self, prelude::*},
    os::unix::process::ExitStatusExt,
    process::{Command, ExitStatus, Stdio},
};

use crate::cmd::Cmd;

pub type InternalFunc = fn(Cmd) -> Result<ExitStatus, CommandError>;
pub type InternalFuncMap = HashMap<String, InternalFunc>;

#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("[io]: {0}")]
    IOError(#[from] io::Error),
    #[error("[{prog_name}]: {message}")]
    Custom {
        prog_name: String,
        message: String,
        status: i32,
    },

    #[error("[{1}]: Unable to launch this program\nCause: {0}")]
    ChildSpawnError(io::Error, String, i32),
    #[error("")]
    ChildExit(io::Error, i32),
}

pub fn clear(_: Cmd) -> Result<ExitStatus, CommandError> {
    print!("\x1b[2J\x1b[H");
    io::stdout().flush()?;
    Ok(ExitStatus::from_raw(0))
}

pub fn cd(Cmd { args, .. }: Cmd) -> Result<ExitStatus, CommandError> {
    if args.is_empty() {
        if let Ok(dir_path) = env::var("HOME") {
            let new_cwd = std::path::Path::new(&dir_path);

            if let Err(e) = env::set_current_dir(&new_cwd) {
                let message = format!("{}", e);
                Err(CommandError::Custom {
                    prog_name: "cmd".into(),
                    message,
                    status: 1,
                })
            } else {
                Ok(ExitStatus::from_raw(0))
            }
        } else {
            eprintln!("cd: No Argument provided");
            Ok(ExitStatus::from_raw(1))
        }
    } else {
        let mut dir_path = args[0].clone();
        if dir_path.contains('~') {
            if let Ok(home) = env::var("HOME") {
                dir_path = dir_path.replace("~", &home);
            }
        }
        let new_cwd = std::path::Path::new(&dir_path);

        if let Err(e) = env::set_current_dir(&new_cwd) {
            let message = format!("{}", e);
            Err(CommandError::Custom {
                prog_name: "cmd".into(),
                message,
                status: 1,
            })
        } else {
            Ok(ExitStatus::from_raw(0))
        }
    }
}

pub fn get_internal_functions_map() -> InternalFuncMap {
    let mut map = InternalFuncMap::new();
    map.insert("clear".into(), clear);
    map.insert("cd".into(), cd);

    map
}

fn resetvars(overrides: HashMap<String, String>, before_run: env::Vars) {
    for (k, _) in overrides {
        env::remove_var(k);
    }

    for (k, v) in before_run {
        env::set_var(k, v);
    }
}

pub fn run(
    Cmd {
        variables_overrides,
        name,
        args,
    }: Cmd,
) -> Result<ExitStatus, CommandError> {
    let previous_vars_state = env::vars();

    for (k, v) in &variables_overrides {
        env::set_var(k, v);
    }

    match Command::new(name.as_str())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .args(args)
        .spawn()
    {
        Ok(mut child) => match child.wait() {
            Ok(status) => {
                resetvars(variables_overrides, previous_vars_state);
                return Ok(status);
            }
            Err(e) => {
                resetvars(variables_overrides, previous_vars_state);
                return Err(CommandError::ChildExit(e, 130));
            }
        },
        Err(e) => {
            resetvars(variables_overrides, previous_vars_state);
            return Err(CommandError::ChildSpawnError(e, name, 127));
        }
    }
}
