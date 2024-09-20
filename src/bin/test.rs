use dsh::cmd::Token;

fn main() {
    eprintln!(
        "{:?}",
        Token::tokenize("THIS_VAR=3333 vim \"I'm a string arg\" $PATH$PATH")
    );
}
