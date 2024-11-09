use std::io;

#[derive(Debug)]
pub enum CmdParsingError {}

impl std::error::Error for CmdParsingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl std::fmt::Display for CmdParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub enum CommandError {
    IOError(io::Error),
    Custom {
        prog_name: String,
        message: String,
        status: i32,
    },
    ChildSpawnError(io::Error, String, i32),
    ChildExit(io::Error, i32),
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::IOError(error) => write!(f, "[io]: {error}"),
            CommandError::Custom {
                prog_name, message, ..
            } => write!(f, "[{prog_name}]: {message}"),
            CommandError::ChildSpawnError(error, name, _status) => {
                write!(f, "[{name}]: Unable to launch this program\nCause: {error}")
            }
            CommandError::ChildExit(_, _) => write!(f, ""),
        }
    }
}
