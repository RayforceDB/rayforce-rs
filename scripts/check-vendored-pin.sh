#!/usr/bin/env bash
# Assert the vendored core submodule matches the pin baked into
# rayforce-sys/build.rs.
#
# The build script stamps CORE_VERSION / CORE_COMMIT into librayforce.a because
# a crate unpacked from crates.io has no git history for the core's Makefile to
# read them from (it resolves them via `git describe` / `git rev-parse`, see
# Makefile:19 and Makefile:27 in the core). If the constants and the submodule
# disagree, published crates report a version they were not built from.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
build_rs="$root/rayforce-sys/build.rs"
core="$root/rayforce-sys/vendor/rayforce"

field() { sed -n "s/^const $1: &str = \"\(.*\)\";\$/\1/p" "$build_rs"; }

want_version="$(field CORE_VERSION)"
want_commit="$(field CORE_COMMIT)"
if [ -z "$want_version" ] || [ -z "$want_commit" ]; then
  echo "could not read CORE_VERSION / CORE_COMMIT from $build_rs" >&2
  exit 1
fi

if [ ! -e "$core/include/rayforce.h" ]; then
  echo "vendored core is missing from $core" >&2
  echo "run: git submodule update --init --recursive" >&2
  exit 1
fi

got_tag="$(git -C "$core" describe --tags --exact-match 2>/dev/null || echo '<not on a tag>')"
got_commit="$(git -C "$core" rev-parse --short="${#want_commit}" HEAD)"

status=0
if [ "$got_tag" != "v$want_version" ]; then
  echo "core submodule is at $got_tag, but build.rs CORE_VERSION expects v$want_version" >&2
  status=1
fi
if [ "$got_commit" != "$want_commit" ]; then
  echo "core submodule is at $got_commit, but build.rs CORE_COMMIT expects $want_commit" >&2
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo "vendored core pin OK: v$want_version ($want_commit)"
fi
exit "$status"
