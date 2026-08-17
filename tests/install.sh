#!/bin/sh

set -eu

repository_root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
test_directory="$(mktemp -d "${TMPDIR:-/tmp}/dome-installer-test.XXXXXX")"
asset="dome-x86_64-unknown-linux-gnu"
release_directory="$test_directory/release"
install_directory="$test_directory/bin"

cleanup() {
    rm -f \
        "$release_directory/$asset" \
        "$release_directory/SHA256SUMS" \
        "$install_directory/dome"
    rmdir "$release_directory" "$install_directory" "$test_directory" 2>/dev/null || true
}

trap cleanup EXIT HUP INT TERM

mkdir -p "$release_directory"
printf '#!/bin/sh\nprintf "fake dome\\n"\n' >"$release_directory/$asset"
chmod 0755 "$release_directory/$asset"
(
    cd "$release_directory"
    sha256sum "$asset" >SHA256SUMS
)

install_from_fixture() {
    DOME_RELEASE_BASE_URL="file://$release_directory" \
        DOME_INSTALL_DIR="$install_directory" \
        sh "$repository_root/install.sh"
}

install_from_fixture
cmp "$release_directory/$asset" "$install_directory/dome"
test -x "$install_directory/dome"

# Reinstalling the same release must remain safe and produce the same bytes.
install_from_fixture
cmp "$release_directory/$asset" "$install_directory/dome"
