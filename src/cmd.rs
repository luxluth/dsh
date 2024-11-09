use std::{collections::HashMap, env, iter::Peekable};

use crate::error::CmdParsingError;

#[derive(Debug)]
pub struct Cmd {
    pub variables_overrides: HashMap<String, String>,
    pub name: String,
    pub args: Vec<String>,
}

enum ParsingState {
    Vars,
    Args,
}

#[derive(Debug)]
enum ArgReplacementState {
    ParsingVar(Vec<char>),
    IsEscaped,
    Next,
}

fn expand_vars_into_arg(override_maps: &HashMap<String, String>, arg: &str) -> String {
    let mut final_arg: String = String::new();
    let mut state = ArgReplacementState::Next;
    for character in arg.chars() {
        if character == '$' {
            match state {
                ArgReplacementState::ParsingVar(xs) => {
                    let var_string = xs
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join("");
                    if let Some(value) = override_maps.get(&var_string) {
                        final_arg.push_str(value);
                    } else {
                        if let Ok(value) = env::var(&var_string) {
                            final_arg.push_str(&value);
                        } else {
                            final_arg.push_str("");
                        }
                    }

                    state = ArgReplacementState::ParsingVar(vec![]);
                }
                ArgReplacementState::IsEscaped => {
                    final_arg.push(character);
                    state = ArgReplacementState::Next;
                }
                ArgReplacementState::Next => {
                    state = ArgReplacementState::ParsingVar(vec![]);
                }
            }
        } else if character == '\\' {
            match state {
                ArgReplacementState::ParsingVar(xs) => {
                    if character.is_ascii_alphanumeric() || character == '_' {
                        let mut xs = xs;
                        xs.push(character);
                        state = ArgReplacementState::ParsingVar(xs);
                    } else {
                        let var_string = xs
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<String>>()
                            .join("");
                        if let Some(value) = override_maps.get(&var_string) {
                            final_arg.push_str(value);
                        } else {
                            if let Ok(value) = env::var(&var_string) {
                                final_arg.push_str(&value);
                            } else {
                                final_arg.push_str("");
                            }
                        }

                        state = ArgReplacementState::IsEscaped;
                    }
                }
                ArgReplacementState::IsEscaped => {
                    final_arg.push(character);
                    state = ArgReplacementState::Next;
                }
                ArgReplacementState::Next => {
                    state = ArgReplacementState::IsEscaped;
                }
            }
        } else {
            match state {
                ArgReplacementState::ParsingVar(xs) => {
                    if character.is_ascii_alphanumeric() || character == '_' {
                        let mut xs = xs;
                        xs.push(character);
                        state = ArgReplacementState::ParsingVar(xs);
                    } else {
                        let var_string = xs
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<String>>()
                            .join("");
                        if let Some(value) = override_maps.get(&var_string) {
                            final_arg.push_str(value);
                        } else {
                            if let Ok(value) = env::var(&var_string) {
                                final_arg.push_str(&value);
                            } else {
                                final_arg.push_str("");
                            }
                        }

                        state = ArgReplacementState::Next;
                    }
                }
                ArgReplacementState::IsEscaped => {
                    final_arg.push(character);
                    state = ArgReplacementState::Next;
                }
                ArgReplacementState::Next => {
                    final_arg.push(character);
                    state = ArgReplacementState::Next;
                }
            }
        }
    }

    match state {
        ArgReplacementState::ParsingVar(xs) => {
            let var_string = xs
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join("");
            if let Some(value) = override_maps.get(&var_string) {
                final_arg.push_str(value);
            } else {
                if let Ok(value) = env::var(&var_string) {
                    final_arg.push_str(&value);
                } else {
                    final_arg.push_str("");
                }
            }
        }
        ArgReplacementState::IsEscaped => {
            // FIXME: syntax error
        }
        ArgReplacementState::Next => {
            // FIXME: unreachable
        }
    }

    final_arg
}

fn string_match(s: &str) -> String {
    if s.starts_with('\'') && s.ends_with('\'') {
        let s = s.trim_start_matches('\'').trim_end_matches('\'');
        s.to_string()
    } else if s.starts_with('"') && s.ends_with('"') {
        let s = s.trim_start_matches('"').trim_end_matches('"');
        s.to_string()
    } else {
        s.to_string()
    }
}

#[test]
fn test_expand_var_simple() {
    let mut overridemap = HashMap::new();
    overridemap.insert("VAR".to_string(), "Nothing".to_string());
    let new_arg = expand_vars_into_arg(&overridemap, "$VAR");

    assert_eq!(new_arg, "Nothing".to_string());
}

#[test]
fn test_expand_var_double() {
    let mut overridemap = HashMap::new();
    overridemap.insert("VAR".to_string(), "Nothing".to_string());
    let new_arg = expand_vars_into_arg(&overridemap, "$VAR$VAR");

    assert_eq!(new_arg, "NothingNothing".to_string());
}

#[test]
fn test_expand_var_escape() {
    let mut overridemap = HashMap::new();
    overridemap.insert("VAR".to_string(), "Nothing".to_string());
    let new_arg = expand_vars_into_arg(&overridemap, "$VAR\\$VAR");

    assert_eq!(new_arg, "Nothing$VAR".to_string());
}

struct CmdParser;

#[derive(Debug)]
pub struct Col(u32);

#[derive(Debug)]
pub enum Sym {
    PIPE,
    EQUAL,
}

#[derive(Debug)]
pub enum Token {
    Word(String, Col),
    Symbol(Sym, Col),
    Str(String, Col),
}

fn make_word(it: &mut Peekable<std::str::Chars<'_>>, col: &mut u32) -> Token {
    let mut word = String::new();
    let mut is_escaped = false;

    let first_char = it.next().unwrap();
    if first_char == '$' || first_char.is_alphanumeric() || first_char == '_' {
        word.push(first_char);
    }

    while let Some(c) = it.peek() {
        if is_escaped {
            word.push(it.next().unwrap());
            is_escaped = false;
            continue;
        }

        if c.is_alphanumeric() || *c == '_' {
            word.push(it.next().unwrap());
        } else if *c == '\\' && !is_escaped {
            *col += 1;
            is_escaped = true;
            it.next();
            continue;
        } else {
            break;
        }
    }

    let len = word.len() as u32;
    let tok = Token::Word(word, Col(*col));
    *col += len;
    return tok;
}

fn build_string(it: &mut Peekable<std::str::Chars<'_>>, col: &mut u32, del: char) -> Token {
    let mut string = String::new();
    let mut is_escaped = false;
    it.next().unwrap();

    while let Some(c) = it.next() {
        if c == del && !is_escaped {
            break;
        }
        if c == '\\' && !is_escaped {
            *col += 1;
            is_escaped = true;
            continue;
        }
        string.push(c);
        is_escaped = false;
    }

    let len = string.len() + 2;
    let tok = Token::Str(string, Col(*col));
    *col += len as u32;
    return tok;
}

impl Token {
    pub fn tokenize(line: &str) -> Result<Vec<Token>, CmdParsingError> {
        let mut it = line.chars().into_iter().peekable();
        let mut tokens = vec![];
        let mut col: u32 = 0;

        while let Some(c) = it.peek() {
            match c {
                ' ' => {
                    col += 1;
                    it.next();
                }
                '=' => {
                    tokens.push(Token::Symbol(Sym::EQUAL, Col(col)));
                    it.next();
                    col += 1;
                }
                '|' => {
                    tokens.push(Token::Symbol(Sym::PIPE, Col(col)));
                    it.next();
                    col += 1;
                }
                '"' => {
                    tokens.push(build_string(&mut it, &mut col, '"'));
                }
                '\'' => {
                    tokens.push(build_string(&mut it, &mut col, '\''));
                }
                _ => {
                    tokens.push(make_word(&mut it, &mut col));
                }
            }
        }

        Ok(tokens)
    }
}

impl CmdParser {
    pub fn parse(line: &str) -> Result<Vec<Cmd>, CmdParsingError> {
        let cmds = vec![];
        let _tokens = Token::tokenize(line)?;

        Ok(cmds)
    }
}

impl Cmd {
    pub fn new(raw_cmd: &str) -> Result<Self, ()> {
        let mut split = raw_cmd.split(' ');
        let mut vars: HashMap<String, String> = HashMap::new();
        let mut name = String::new();
        let mut state: ParsingState = ParsingState::Vars;
        let mut args: Vec<String> = vec![];

        while let Some(part) = split.next() {
            match state {
                ParsingState::Vars => {
                    if part.contains('=') {
                        if let Some((var, value)) = part.split_once('=') {
                            vars.insert(var.into(), value.into());
                        } else {
                            // Error occured while parsing the args
                        }
                    } else {
                        name = part.into();
                        state = ParsingState::Args;
                    }
                }
                ParsingState::Args => {
                    // TODO: Expand "*" "~" and "$VARNAME"
                    // Expntion to $(echo "lol")
                    // bash extensions, etc, ..
                    let mut temp_args = vec![expand_vars_into_arg(&vars, &string_match(part))];

                    temp_args.extend(
                        split
                            .map(|arg| expand_vars_into_arg(&vars, &string_match(arg)))
                            .collect::<Vec<String>>(),
                    );
                    args.extend(temp_args);
                    break;
                }
            }
        }

        Ok(Self {
            variables_overrides: vars,
            name,
            args,
        })
    }
}
