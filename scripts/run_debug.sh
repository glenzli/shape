#!/bin/sh

set -eu

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_directory/.." && pwd)
repository_parent=$(dirname -- "$repository_root")
local_build_root=${SHAPE_LOCAL_BUILD_ROOT:-"$repository_parent/.shape-local-build"}
canonical_app="$local_build_root/current-debug/Shape.app"
shape_executable="$canonical_app/Contents/MacOS/shape-desktop"
debug_log_root=${SHAPE_DEBUG_LOG_ROOT:-"$local_build_root/logs"}
debug_log="$debug_log_root/shape-debug.log"
project_path=${SHAPE_DEBUG_PROJECT_PATH:-}

usage() {
    cat <<'EOF'
usage:
  ./scripts/run_debug.sh [project.shape]
  ./scripts/run_debug.sh --foreground [project.shape]
  ./scripts/run_debug.sh --check

By default Shape starts in the background from the canonical debug build and
this script returns immediately. Use --foreground to keep Shape attached to the
terminal. Pass a .shape project to open it directly; without one, Shape opens
its empty project shell. SHAPE_DEBUG_PROJECT_PATH supplies a reusable default.
EOF
}

if [ ! -x "$shape_executable" ]; then
    echo "Shape has no promoted canonical debug build." >&2
    echo "Expected: $canonical_app" >&2
    echo "Run ./scripts/build_and_promote_debug.sh first." >&2
    exit 69
fi
if [ ! -r "$canonical_app/Contents/Info.plist" ]; then
    echo "Shape canonical debug build has no bundle metadata." >&2
    echo "Expected: $canonical_app/Contents/Info.plist" >&2
    exit 69
fi
if command -v plutil >/dev/null 2>&1; then
    plutil -lint "$canonical_app/Contents/Info.plist" >/dev/null
fi

if [ "${1:-}" = "--check" ]; then
    [ "$#" -eq 1 ] || { echo "--check does not accept extra arguments" >&2; exit 64; }
    echo "canonical debug app: $canonical_app"
    echo "executable: $shape_executable"
    echo "default project: ${project_path:-<none>}"
    echo "log: $debug_log"
    exit 0
fi

foreground=false
case "${1:-}" in
    --foreground)
        foreground=true
        shift
        ;;
    -h|--help)
        usage
        exit 0
        ;;
esac

[ "$#" -le 1 ] || { usage >&2; exit 64; }
if [ "$#" -eq 1 ]; then
    project_path=$1
fi
if [ -n "$project_path" ]; then
    if [ ! -d "$project_path" ]; then
        echo "Shape debug project does not exist: $project_path" >&2
        exit 66
    fi
    project_directory=$(CDPATH= cd -- "$(dirname -- "$project_path")" && pwd)
    project_path="$project_directory/$(basename -- "$project_path")"
fi

if [ "$foreground" = true ]; then
    if [ -n "$project_path" ]; then
        exec "$shape_executable" --project "$project_path"
    fi
    exec "$shape_executable"
fi

mkdir -p "$debug_log_root"
if [ -n "$project_path" ]; then
    nohup "$shape_executable" --project "$project_path" >>"$debug_log" 2>&1 </dev/null &
else
    nohup "$shape_executable" >>"$debug_log" 2>&1 </dev/null &
fi
shape_pid=$!
echo "Shape started in the background (pid $shape_pid)."
echo "app: $canonical_app"
if [ -n "$project_path" ]; then
    echo "project: $project_path"
fi
echo "log: $debug_log"
