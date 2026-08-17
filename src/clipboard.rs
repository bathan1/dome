use std::io::{self, Read};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};

/// The PowerShell program deliberately reads standard input itself.
///
/// Passing clipboard text as part of `-Command` would require us to escape it
/// as PowerShell source code. A pipe has no such quoting problem, and setting
/// `InputEncoding` makes the byte contract explicit: `clipme` sends UTF-8.
const SET_CLIPBOARD_SCRIPT: &str = r#"
[Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::In.ReadToEnd() | Set-Clipboard
"#;

/// Copy every byte from `input` to the Windows clipboard as UTF-8 text.
///
/// `impl Read` is useful here: callers may supply a file, locked stdin, an
/// in-memory `Cursor`, or any future type implementing the standard `Read`
/// trait without this function knowing its concrete type.
pub fn copy_to_windows_clipboard(input: impl Read) -> io::Result<()> {
    // HOMEWORK 2
    //
    // Connect `input` to PowerShell, close its stdin, and wait for its exit
    // status. The helper functions below demonstrate the obscure APIs; your
    // job is to put them in the correct ownership/lifecycle order.
    //
    // 1. Move the pipe out with `child.stdin.take()`. Convert an unexpected
    //    `None` into `io::Error::other(...)`.
    // 2. Call `stream_to_child(input, child_stdin)`.
    // 3. Call `child.wait()` even if streaming failed, so the process is
    //    always reaped. Keep the streaming result so you can return it first.
    // 4. Pass the exit status to `successful_status`.
    let mut child = spawn_powershell()?;
    let child_stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("powershell.exe stdin was not piped"))?;
    let stream_result = stream_to_child(input, child_stdin);
    let wait_result = child.wait();
    match stream_result {
        Err(error) => return Err(error),
        Ok(()) => match wait_result {
            Err(error) => return Err(error),
            Ok(status) => successful_status(status)
        }
    }
}

/// Spawn the Windows-side process with a writable stdin pipe.
///
/// `Command` is Rust's subprocess builder. `Stdio::piped()` is the important
/// non-obvious setting: without it, `child.stdin` will be `None`.
fn spawn_powershell() -> io::Result<Child> {
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            SET_CLIPBOARD_SCRIPT,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("could not start powershell.exe: {error}"),
            )
        })
}

/// Stream without buffering the entire input in memory.
///
/// Taking `ChildStdin` by value is intentional: it is automatically dropped
/// and therefore closed when this function returns. That EOF tells
/// `[Console]::In.ReadToEnd()` to finish.
fn stream_to_child(mut input: impl Read, mut child_stdin: ChildStdin) -> io::Result<()> {
    // `io::copy` handles its own fixed-size buffer, partial reads, and partial
    // writes. Mapping its byte count to `()` says we only care about success.
    io::copy(&mut input, &mut child_stdin).map(|_bytes_written| ())
}

/// Translate a subprocess exit status into Rust's conventional `Result`.
fn successful_status(status: ExitStatus) -> io::Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "powershell.exe failed with {status}"
        )))
    }
}
