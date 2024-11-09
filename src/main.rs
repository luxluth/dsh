use dsh::{
    // cmd,
    // error::CommandError,
    internals::{
        // self,
        get_internal_functions_map,
    },
};
use nix::sys::signal::{self, SigHandler, Signal};
use std::{
    env,
    io::{self, prelude::*},
    os::fd::AsRawFd,
    sync::atomic::{AtomicBool, AtomicI32, Ordering},
    usize,
};
use termion::{
    event::{
        Event,
        Key,
        // MouseEvent
    },
    input::TermRead,
    raw::IntoRawMode,
};
use tinytoken::Tokenizer;

static NEED_STOP: AtomicBool = AtomicBool::new(false);
static STDIN_FD: AtomicI32 = AtomicI32::new(0);
static CAN_STOP: AtomicBool = AtomicBool::new(true);

struct Shell {
    internals: Option<dsh::internals::InternalFuncMap>,
}

extern "C" fn handle_sighup(signal: libc::c_int) {
    let signal = Signal::try_from(signal).unwrap();
    if CAN_STOP.load(Ordering::Relaxed) {
        NEED_STOP.store(signal == Signal::SIGHUP, Ordering::Relaxed);
        std::process::exit(0);
    }
}

extern "C" fn handle_sigint(_signal: libc::c_int) {
    // let signal = Signal::try_from(signal).unwrap();
    // NEED_STOP.store(signal == Signal::SIGINT, Ordering::Relaxed);
    // std::process::exit(0);
}

struct KeyModifiers {
    alt: bool,
    ctrl: bool,
    shift: bool,
}

impl KeyModifiers {
    pub fn new() -> Self {
        Self {
            alt: false,
            ctrl: false,
            shift: false,
        }
    }
}

struct TextBuffer {
    _capacity: usize,
    _buf: Vec<char>,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            _capacity: 2048,
            _buf: Vec::with_capacity(2048),
        }
    }

    pub fn insert(&mut self, index: usize, element: char) {
        if index > self._capacity {
            if index - self._capacity > 512 {
                self._capacity += index - self._capacity + 512;
            } else {
                self._capacity += 512;
            }
            self._buf.resize_with(self._capacity, Default::default);
        }

        self._buf.insert(index, element);
    }

    pub fn remove(&mut self, index: usize) -> char {
        self._buf.remove(index)
    }

    pub fn len(&self) -> usize {
        return self._buf.len();
    }

    pub fn clear(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self._capacity = 2048;
        self._buf = Vec::with_capacity(self._capacity);
    }
}

impl ToString for TextBuffer {
    fn to_string(&self) -> String {
        let mut str_dupa = String::new();
        for ch in &self._buf {
            str_dupa.push(*ch);
        }
        str_dupa
    }
}

impl Shell {
    const fn new() -> Self {
        Self { internals: None }
    }

    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.internals = Some(get_internal_functions_map());

        let stdin = io::stdin();
        STDIN_FD.store(stdin.as_raw_fd(), Ordering::Relaxed);
        let mut stdout = io::stdout().into_raw_mode()?;
        let hostname_file = std::path::Path::new("/etc/hostname");
        if hostname_file.exists() {
            let mut f = std::fs::File::open(hostname_file).unwrap();
            let mut content = String::new();
            let _ = f.read_to_string(&mut content);

            if !content.is_empty() {
                env::set_var("hostname", content.trim());
            }
        }

        let error_code = 0;
        let mut modifiers = KeyModifiers::new();

        let mut prompt = format!(
            "{}@{} [{}] ",
            env::var("USERNAME").unwrap_or("".to_string()),
            env::var("hostname").unwrap_or("".to_string()),
            error_code,
        );

        write!(
            stdout,
            "{}{}{prompt}",
            termion::cursor::SteadyBar,
            termion::cursor::BlinkingBar,
        )
        .unwrap();
        stdout.flush()?;
        let mut cmd_buff = TextBuffer::new();
        let mut cursor_position = 0u16;

        for c in stdin.events() {
            let ev = c.unwrap();
            match ev {
                Event::Key(key) => match key {
                    Key::Backspace => {
                        if cursor_position > 0 {
                            cmd_buff.remove((cursor_position - 1) as usize);
                            cursor_position -= 1;

                            write!(stdout, "\r{}{}", termion::clear::CurrentLine, prompt)?;
                            write!(stdout, "{}", cmd_buff.to_string())?;

                            let move_left = cmd_buff.len() - cursor_position as usize;
                            if move_left > 0 {
                                write!(stdout, "{}", termion::cursor::Left(move_left as u16))?;
                            }
                        }
                    }
                    Key::Left => {
                        if cursor_position > 0 {
                            cursor_position -= 1;
                            let _ = write!(stdout, "{}", termion::cursor::Left(1));
                        }
                    }
                    // Key::ShiftLeft => todo!(),
                    // Key::AltLeft => todo!(),
                    // Key::CtrlLeft => todo!(),
                    Key::Right => {
                        if cursor_position < cmd_buff.len() as u16 {
                            cursor_position += 1;
                            let _ = write!(stdout, "{}", termion::cursor::Right(1));
                        }
                    }
                    // Key::ShiftRight => todo!(),
                    // Key::AltRight => todo!(),
                    // Key::CtrlRight => todo!(),
                    // Key::Up => todo!(),
                    Key::ShiftUp => {
                        modifiers.shift = false;
                    }
                    Key::AltUp => {
                        modifiers.alt = false;
                    }
                    Key::CtrlUp => {
                        modifiers.ctrl = false;
                    }
                    // Key::Down => todo!(),
                    Key::ShiftDown => {
                        modifiers.shift = true;
                    }
                    Key::AltDown => {
                        modifiers.alt = true;
                    }
                    Key::CtrlDown => {
                        modifiers.ctrl = true;
                    }
                    Key::Home => {
                        cursor_position = 0;
                    }
                    // Key::CtrlHome => todo!(),
                    Key::End => {
                        cursor_position = (cmd_buff.len() - 1) as u16;
                    }
                    // Key::CtrlEnd => todo!(),
                    // Key::PageUp => todo!(),
                    // Key::PageDown => todo!(),
                    // Key::BackTab => todo!(),
                    // Key::Delete => todo!(),
                    // Key::Insert => todo!(),
                    // Key::F(_) => todo!(),
                    Key::Char(ch) => {
                        if ch == '\n' {
                            let _ = write!(stdout, "\n");
                            let _tokens = Tokenizer::builder()
                                .ignore_numbers(true)
                                .parse_char_as_string(true)
                                .add_symbol('=')
                                .build(&cmd_buff.to_string())
                                .tokenize()?;
                            cmd_buff.clear();
                            stdout.flush()?;
                            let display_length = prompt.len() as u16 + cursor_position;
                            cursor_position = 0;
                            let _ = write!(
                                stdout,
                                "{}{}",
                                termion::clear::CurrentLine,
                                termion::cursor::Left(display_length)
                            );
                            stdout.flush()?;
                            prompt = format!(
                                "{}@{} [{}] ",
                                env::var("USERNAME").unwrap_or("".to_string()),
                                env::var("hostname").unwrap_or("".to_string()),
                                error_code,
                            );
                            write!(stdout, "{}{}", termion::clear::CurrentLine, prompt)?;
                            // let _ = write!(
                            //     stdout,
                            //     "{prompt}{}",
                            //     termion::cursor::Right(prompt.len() as u16)
                            // );
                        } else if ch == '\t' {
                            // Handle completion
                        } else {
                            let display_length = prompt.len() as u16 + cursor_position;
                            let _ = write!(
                                stdout,
                                "{}{}{prompt}",
                                termion::clear::CurrentLine,
                                termion::cursor::Left(display_length)
                            );
                            stdout.flush()?;
                            cmd_buff.insert(cursor_position as usize, ch);
                            cursor_position += 1;
                            let _ = write!(stdout, "{}", cmd_buff.to_string());
                            stdout.flush()?;
                            let move_left = cmd_buff.len() - cursor_position as usize;
                            if move_left > 0 {
                                let _ =
                                    write!(stdout, "{}", termion::cursor::Left(move_left as u16));
                            }
                        }
                    }
                    // Key::Alt(_) => todo!(),
                    Key::Ctrl(c) => {
                        if c.to_lowercase().to_string() == "d" {
                            break;
                        }
                    }
                    Key::Null => {
                        break;
                    }
                    _ => {}
                },
                Event::Mouse(_mouse_event) => {}
                Event::Unsupported(_vec) => {}
            }
            let _ = stdout.flush();
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hup_handler = SigHandler::Handler(handle_sighup);
    let int_handler = SigHandler::Handler(handle_sigint);
    unsafe { signal::signal(Signal::SIGHUP, hup_handler) }.unwrap();
    unsafe { signal::signal(Signal::SIGINT, int_handler) }.unwrap();
    let mut shell = Shell::new();
    shell.run()?;

    Ok(())
}
