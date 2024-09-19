use std::{collections::HashMap, env};

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
