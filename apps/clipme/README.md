# ClipMe

`clipme` copies exact UTF-8 text from WSL to the Windows clipboard. It requires
`powershell.exe` to be reachable from WSL.

## Install

Install ClipMe and optionally its Codex or Claude Code skill through Dome:

```console
dome add clipme
```

Dome installs the binary into `${CARGO_HOME:-$HOME/.cargo}/bin` and prompts for
the agents that should receive the bundled skill. The default skill roots are
`~/.agents/skills` for Codex and `~/.claude/skills` for Claude Code. Re-running
the command updates Dome-managed files without overwriting an unrelated skill.

Remove the binary and its Dome-managed skills with:

```console
dome remove clipme
```

Or install it from a checkout:

```console
cargo install --path apps/clipme --locked
```

To install the published binary without Dome, download it together with the
release checksums:

```sh
release_dir="$(mktemp -d)"
curl --fail --location \
  --output "$release_dir/clipme-x86_64-unknown-linux-gnu" \
  https://github.com/bathan1/dome/releases/latest/download/clipme-x86_64-unknown-linux-gnu
curl --fail --location \
  --output "$release_dir/SHA256SUMS" \
  https://github.com/bathan1/dome/releases/latest/download/SHA256SUMS
(cd "$release_dir" && sha256sum --check --ignore-missing SHA256SUMS)
install -Dm755 "$release_dir/clipme-x86_64-unknown-linux-gnu" \
  "${CARGO_HOME:-$HOME/.cargo}/bin/clipme"
```

## Usage

Pass one-line text as arguments:

```console
clipme 'hello, 世界 👋'
```

Pipe exact or multiline bytes through standard input. ClipMe does not append a
newline:

```console
printf '%s' 'hello, 世界 👋' | clipme
```

Arguments take precedence over standard input. Calling `clipme` with no
arguments from an interactive terminal returns an error rather than clearing
the clipboard.

The agent skill distributed with ClipMe lives in [`skill/`](skill/).
