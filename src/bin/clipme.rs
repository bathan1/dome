use std::env;
use std::io::{self, Cursor, IsTerminal};
use std::process::ExitCode;

use dome::clipboard::copy_to_windows_clipboard;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("clipme: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    // Arguments take precedence even when an agent launches us with a
    // non-terminal stdin. This makes both `clipme hello` and a pipeline work
    // reliably in interactive shells and non-interactive agent processes.
    if let Some(text) = join_arguments_text(env::args().skip(1)) {
        return copy_to_windows_clipboard(Cursor::new(text.into_bytes()));
    }

    let stdin = io::stdin();
    if stdin.is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "provide text as arguments or pipe UTF-8 text on stdin",
        ));
    }

    copy_to_windows_clipboard(stdin.lock())
}

/// `join_arguments_text(arguments)` joins each text entry in ARGUMENTS with a space to rebuild
/// the original text.
fn join_arguments_text<It>(arguments: It) -> Option<String>
where It: Iterator<Item = String>
{
    let arguments: Vec<String> = arguments.collect();
    return match arguments.as_slice() {
        [] => None,
        xs @ [..] => Some (xs.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::join_arguments_text;

    #[test]
    fn arguments_are_joined_like_zsh_dollar_star() {
        let arguments = ["hello", "from", "Rust"].map(String::from).into_iter();
        assert_eq!(
            join_arguments_text(arguments),
            Some(String::from("hello from Rust"))
        );
    }

    #[test]
    fn no_arguments_selects_standard_input() {
        assert_eq!(join_arguments_text(std::iter::empty()), None);
    }
}
