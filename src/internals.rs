use std::{
    collections::HashMap,
    io::{self, prelude::*},
    os::unix::process::ExitStatusExt,
    process::{Command, ExitStatus, Stdio},
};

use crate::cmd::Cmd;

pub type InternalFunc = fn(Cmd) -> Result<ExitStatus, Box<dyn std::error::Error>>;
pub type InternalFuncMap = HashMap<String, InternalFunc>;

pub fn clear(_: Cmd) -> Result<ExitStatus, Box<dyn std::error::Error>> {
    print!("\x1b[2J\x1b[H");
    io::stdout().flush()?;
    Ok(ExitStatus::from_raw(0))
}

pub fn get_internal_functions_map() -> InternalFuncMap {
    let mut map = InternalFuncMap::new();
    map.insert("clear".into(), clear);

    map
}

pub fn run(
    Cmd {
        variables_overrides,
        name,
        args,
    }: Cmd,
) -> Result<ExitStatus, Box<dyn std::error::Error>> {
    if let Ok(mut child) = Command::new(name.as_str())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .args(args)
        .spawn()
    {
        if let Ok(status) = child.wait() {
            return Ok(status);
        } else {
            return Ok(ExitStatus::from_raw(127));
        }
    } else {
        return Ok(ExitStatus::from_raw(127));
    }
}
