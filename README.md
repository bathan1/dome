# dome (`do-me`)

`dome` is a small installer and a collection of agent-friendly action commands.
The first action is `clipme`, which copies exact UTF-8 text from WSL to the
Windows clipboard.

The published binaries currently target x86-64 Linux/WSL. `clipme` requires
`powershell.exe` to be reachable from WSL.

## Install Dome

Install the latest release with either `curl`:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://github.com/bathan1/dome/releases/latest/download/install.sh | sh
```

or `wget`:

```sh
wget -qO- https://github.com/bathan1/dome/releases/latest/download/install.sh | sh
```

The installer downloads the `dome` binary from the latest GitHub release,
verifies it against that release's `SHA256SUMS`, and installs it to
`${CARGO_HOME:-$HOME/.cargo}/bin`. Set `DOME_INSTALL_DIR` to use a different
directory, or `DOME_VERSION` to install a specific tag. Make sure the selected
directory is on the `PATH` inherited by the agent process, not only an
interactive shell.

Then install ClipMe and its agent skills:

```console
dome add clipme
```

`dome add` downloads and verifies the latest ClipMe release, installs it into
`${CARGO_HOME:-$HOME/.cargo}/bin`, and opens an interactive multi-select for
agent integrations. The current choices are Codex and Claude Code. For every
selected agent, Dome prompts for a skills root and defaults to:

```text
Codex:       ~/.agents/skills
Claude Code: ~/.claude/skills
```

The resulting skill is installed below that root as `clipme/SKILL.md`. Running
the same command again is safe: the binary and Dome-managed skills are updated
only to the latest published content. Dome refuses to overwrite a different,
unmanaged skill.

Remove the binary and every Dome-managed skill recorded for it with:

```console
dome remove clipme
```

Removal is also idempotent and leaves unmanaged files alone.

## Install ClipMe manually

ClipMe remains independently installable from the same release:

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

Use arguments for convenient one-line text:

```console
clipme 'hello, 世界 👋'
```

Use standard input for exact or multiline bytes. ClipMe does not append a
newline:

```console
printf '%s' 'hello, 世界 👋' | clipme
```

Arguments take precedence over standard input. Calling `clipme` with no
arguments from an interactive terminal returns an error instead of clearing the
clipboard.

## Build from source

```console
cargo fmt --all --check
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked --bins
```

Install either binary from a checkout:

```console
cargo install --path . --locked --bin dome
cargo install --path . --locked --bin clipme
```

## Publish a release

The release workflow builds, tests, and publishes both binaries, the one-line
installer, and `SHA256SUMS` whenever a semantic version tag is pushed:

```console
git tag v0.2.0
git push origin v0.2.0
```

The GitHub repository must be public before unauthenticated `dome add` requests
can download its release assets.
