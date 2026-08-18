#!/bin/sh

set -eu

repository_root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
test_directory="$(mktemp -d "${TMPDIR:-/tmp}/squid-installer-test.XXXXXX")"
asset="squid-x86_64-unknown-linux-gnu"
release_directory="$test_directory/release"
install_directory="$test_directory/bin"

cleanup() {
    rm -f \
        "$release_directory/$asset" \
        "$release_directory/SHA256SUMS" \
        "$install_directory/squid"
    rmdir "$release_directory" "$install_directory" "$test_directory" 2>/dev/null || true
}

trap cleanup EXIT HUP INT TERM

mkdir -p "$release_directory"
printf '#!/bin/sh\nprintf "fake squid\\n"\n' >"$release_directory/$asset"
chmod 0755 "$release_directory/$asset"
(
    cd "$release_directory"
    sha256sum "$asset" >SHA256SUMS
)

install_from_fixture() {
    DOME_RELEASE_BASE_URL="file://$release_directory" \
        SQUID_INSTALL_DIR="$install_directory" \
        sh "$repository_root/apps/squid/scripts/install.sh"
}

install_from_fixture
cmp "$release_directory/$asset" "$install_directory/squid"
test -x "$install_directory/squid"

# Reinstalling the same release must remain safe and produce the same bytes.
install_from_fixture
cmp "$release_directory/$asset" "$install_directory/squid"
