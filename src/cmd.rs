use std::collections::HashMap;

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
                    let mut temp_args = vec![part.to_string()];

                    temp_args.extend(split.map(|arg| arg.to_string()).collect::<Vec<String>>());
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
