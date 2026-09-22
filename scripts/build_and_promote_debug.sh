#!/bin/sh

# Build the user-launchable Shape debug product, validate its packaged startup
# path, and atomically advance the stable current-debug application.
set -eu

usage() {
    cat <<'EOF'
usage:
  ./scripts/build_and_promote_debug.sh [validation-label]
  ./scripts/build_and_promote_debug.sh --quick [validation-label]
  ./scripts/build_and_promote_debug.sh --full [validation-label]
  ./scripts/build_and_promote_debug.sh --check

Builds Shape.app in one stable external candidate directory, validates the
packaged desktop smoke paths, then atomically promotes the bundle to
current-debug.

The default and --quick modes run the translation completeness check, build the
desktop bundle, and run the packaged smoke paths. Use --full before a commit,
cross-module integration handoff, or release-like checkpoint; it additionally
runs Rust formatting, workspace Clippy, and the full Rust workspace test suite.

Optional overrides:
  SHAPE_CANONICAL_DEBUG_BUILD_DIR         External CMake candidate directory.
  SHAPE_CANONICAL_DEBUG_CARGO_TARGET_DIR  External Cargo target directory.
  SHAPE_LOCAL_BUILD_ROOT                  Root containing releases and current-debug.
EOF
}

fail() {
    echo "canonical debug build: $*" >&2
    exit 69
}

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_directory/.." && pwd)
repository_parent=$(dirname -- "$repository_root")
local_build_root=${SHAPE_LOCAL_BUILD_ROOT:-"$repository_parent/.shape-local-build"}
build_directory=${SHAPE_CANONICAL_DEBUG_BUILD_DIR:-"$local_build_root/canonical-debug-build"}
cargo_target_directory=${SHAPE_CANONICAL_DEBUG_CARGO_TARGET_DIR:-"$repository_parent/.shape-local-target/canonical-debug"}
candidate_app="$build_directory/apps/desktop/shape-desktop.app"
candidate_executable="$candidate_app/Contents/MacOS/shape-desktop"

case "$build_directory" in
    /*) ;;
    *) fail "SHAPE_CANONICAL_DEBUG_BUILD_DIR must be an absolute path" ;;
esac
case "$cargo_target_directory" in
    /*) ;;
    *) fail "SHAPE_CANONICAL_DEBUG_CARGO_TARGET_DIR must be an absolute path" ;;
esac
case "$build_directory" in
    "$repository_root"|"$repository_root"/*) fail "candidate build directory must stay outside the source tree" ;;
esac
case "$cargo_target_directory" in
    "$repository_root"|"$repository_root"/*) fail "Cargo target directory must stay outside the source tree" ;;
esac

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    usage
    exit 0
fi
if [ "${1:-}" = "--check" ]; then
    [ "$#" -eq 1 ] || fail "--check does not accept a validation label"
    echo "candidate build directory: $build_directory"
    echo "candidate app: $candidate_app"
    echo "canonical app: $local_build_root/current-debug/Shape.app"
    echo "default validation: quick (use --full for repository-wide Rust checks)"
    exit 0
fi

validation_mode=quick
case "${1:-}" in
    --quick)
        shift
        ;;
    --full)
        validation_mode=full
        shift
        ;;
esac
[ "$#" -le 1 ] || { usage >&2; exit 64; }

validation_label=${1:-"canonical-debug-$(git -C "$repository_root" rev-parse --short HEAD)"}

if grep -q 'type="unfinished"' "$repository_root/apps/desktop/translations/shape_zh_CN.ts"; then
    fail "Simplified Chinese translations contain unfinished messages"
fi

if [ "$validation_mode" = full ]; then
    (
        cd "$repository_root"
        cargo fmt --all -- --check
        CARGO_TARGET_DIR="$cargo_target_directory" cargo clippy --workspace --all-targets -- -D warnings
        CARGO_TARGET_DIR="$cargo_target_directory" cargo test --workspace
    )
fi
cmake --preset desktop-dev -B "$build_directory"
cmake --build "$build_directory" --target shape-desktop --parallel 6

if [ ! -x "$candidate_executable" ]; then
    fail "desktop build produced no executable candidate: $candidate_executable"
fi
ctest --test-dir "$build_directory" --output-on-failure \
    -R '^shape-desktop-(smoke|project-smoke)$'

"$repository_root/scripts/promote_debug_build.sh" "$candidate_app" "$validation_mode-$validation_label"
"$repository_root/scripts/run_debug.sh" --check
