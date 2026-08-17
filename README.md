# dome (`do-me`)

`dome` is a collection of small, agent-friendly commands named as actions:
`clipme`, and eventually any other `*me` workflows that earn a place here.

The first action, `clipme`, is the learning exercise. Its requirements, process
setup, UTF-8 PowerShell script, tests, and less-discoverable standard-library
helpers are provided. You fill in the two `todo!()` checkpoints that connect
the pieces.

## The `clipme` assignment

Implement these behaviors:

1. When arguments are present, join them with a single space and copy that
   text. Explicit arguments win even in a non-interactive agent process.
2. With no arguments and piped stdin, copy stdin exactly without adding a
   newline.
3. With no arguments in an interactive terminal, return a useful error instead
   of unexpectedly clearing the clipboard.
4. Start `powershell.exe`, stream the input bytes to its stdin, close the pipe,
   wait for it, and turn a failed exit status into a Rust error.
5. Keep the PowerShell input encoding explicitly set to BOM-less UTF-8.

Start with the tests. They intentionally fail at `HOMEWORK 1`:

```console
cargo test --bin clipme
```

Search `src` for the work sites:

```console
rg 'HOMEWORK|todo!' src
```

After implementing both checkpoints, format, test, lint, and try both input
styles:

```console
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --bin clipme
target/debug/clipme 'hello, 世界 👋'
printf '%s' 'hello, 世界 👋' | target/debug/clipme
```

Arguments are joined with a single space, matching the useful behavior of
zsh's `"$*"`. For multiline text, leading/trailing whitespace, or shell-special
characters, pipe the exact bytes on stdin. `clipme` does not append a newline.

Only install it for agents after your implementation passes:

```console
cargo install --path . --bin clipme
command -v clipme
```

Cargo normally installs it as `$CARGO_HOME/bin/clipme` (usually
`~/.cargo/bin/clipme`). That directory must be on the `PATH` inherited by the
agent process—not only added inside an interactive-only zsh configuration.

Then an agent can be told simply:

> Write the release notes to `clipme`.

The most robust generated command is a byte-preserving stdin pipe into the
binary; the binary owns the Windows/UTF-8 details.

## How the project is organized

```text
Cargo.toml                 package metadata
src/lib.rs                 shared code exported by the `dome` crate
src/clipboard.rs           HOMEWORK 2 and reusable clipboard integration
src/bin/clipme.rs          HOMEWORK 1 and the `clipme` CLI behavior
```

Every Rust file directly under `src/bin/` becomes a separate executable. If
you add `src/bin/slugme.rs`, Cargo automatically gives you these commands:

```console
cargo run --bin slugme
cargo build --release --bin slugme
cargo install --path . --bin slugme
```

Code shared by several commands belongs under `src/` and is exported from
`src/lib.rs`. Keeping the thin user interface in `src/bin` and the mechanism in
the library also makes the mechanism easier to test.

The source demonstrates standard-library pieces that are otherwise hard to
guess on day one: `IsTerminal` distinguishes a terminal from a pipeline;
`Cursor` makes in-memory UTF-8 bytes implement `Read`; generic `impl Read`
accepts stdin, files, and memory without knowing the concrete type; `Command`
and `Stdio::piped` create a child process with a writable pipe; `io::copy`
handles partial reads and writes; and `ExitCode` lets `main` report failure
without terminating from inside reusable code.

## What happens during a build

1. Cargo reads `Cargo.toml`, discovers the library and every file in `src/bin`.
2. `rustc` compiles `src/lib.rs` as the reusable `dome` crate.
3. `rustc` compiles `clipme.rs` and links it against that crate.
4. A debug build lands at `target/debug/clipme`. `todo!()` is valid Rust, so the
   starter compiles, but execution panics if it reaches unfinished homework.
5. `cargo build --release` recompiles with optimizations and places the smaller,
   faster binary at `target/release/clipme`.
6. `cargo install --path . --bin clipme` performs a release build and copies the
   resulting executable to Cargo's user-level binary directory.

The Rust binary still delegates the final clipboard call to `powershell.exe`,
because the destination is the Windows clipboard. It passes text over stdin,
sets PowerShell's input encoding to BOM-less UTF-8, closes the pipe to signal
EOF, waits for PowerShell, and returns a failure exit status if anything fails.
