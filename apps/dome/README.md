# Dome

`dome` installs and removes binaries from the Dome suite. Published binaries
currently target x86-64 Linux and WSL.

## Install

```sh
curl --proto '=https' --tlsv1.2 -fsSL \
  https://github.com/bathan1/dome/releases/latest/download/install.sh | sh
```

The installer writes to `${CARGO_HOME:-$HOME/.cargo}/bin`. Set
`DOME_INSTALL_DIR` to choose another directory or `DOME_VERSION` to install a
specific tag.

## Usage

```console
dome add clipme
dome add squid
dome remove clipme
dome remove squid
```

Every download is verified against the release's `SHA256SUMS`. ClipMe also
offers an interactive choice of agent skills to install for Codex or Claude
Code. Repeated additions and removals are safe, and Dome does not overwrite
unmanaged skills.

## Development

From the workspace root:

```console
cargo fmt --all --check
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --release --locked --bins
```

Install the workspace package directly with:

```console
cargo install --path apps/dome --locked
```
