#!/bin/sh

set -eu

repository="bathan1/dome"
asset="squid-x86_64-unknown-linux-gnu"
install_directory="${SQUID_INSTALL_DIR:-${CARGO_HOME:-$HOME/.cargo}/bin}"

fail() {
    printf 'squid installer: %s\n' "$1" >&2
    exit 1
}

for command_name in uname mktemp awk sha256sum install; do
    command -v "$command_name" >/dev/null 2>&1 || fail "required command not found: $command_name"
done

[ "$(uname -s)" = "Linux" ] || fail "published binaries currently support Linux/WSL only"

case "$(uname -m)" in
    x86_64 | amd64) ;;
    *) fail "published binaries currently support x86-64 only" ;;
esac

if [ -n "${DOME_RELEASE_BASE_URL:-}" ]; then
    release_base_url="${DOME_RELEASE_BASE_URL%/}"
elif [ -n "${SQUID_VERSION:-}" ]; then
    release_base_url="https://github.com/$repository/releases/download/$SQUID_VERSION"
else
    release_base_url="https://github.com/$repository/releases/latest/download"
fi

download() {
    source_url="$1"
    destination="$2"

    if command -v curl >/dev/null 2>&1; then
        curl --fail --location --silent --show-error --output "$destination" "$source_url"
    elif command -v wget >/dev/null 2>&1; then
        wget --quiet --output-document="$destination" "$source_url"
    else
        fail "curl or wget is required"
    fi
}

temporary_directory="$(mktemp -d "${TMPDIR:-/tmp}/squid-install.XXXXXX")"
cleanup() {
    rm -f \
        "$temporary_directory/$asset" \
        "$temporary_directory/SHA256SUMS" \
        "$temporary_directory/SQUID_SHA256"
    rmdir "$temporary_directory" 2>/dev/null || true
}
trap cleanup EXIT HUP INT TERM

download "$release_base_url/$asset" "$temporary_directory/$asset"
download "$release_base_url/SHA256SUMS" "$temporary_directory/SHA256SUMS"

awk -v asset="$asset" '$2 == asset { print }' \
    "$temporary_directory/SHA256SUMS" >"$temporary_directory/SQUID_SHA256"
[ -s "$temporary_directory/SQUID_SHA256" ] || fail "SHA256SUMS has no entry for $asset"

(
    cd "$temporary_directory"
    sha256sum --check SQUID_SHA256
)

install -d "$install_directory"
install -m 0755 "$temporary_directory/$asset" "$install_directory/squid"

printf 'Installed Squid to %s/squid\n' "$install_directory"
