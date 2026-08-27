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

head_commit="$(git -C "$core" rev-parse HEAD)"
got_commit="$(git -C "$core" rev-parse --short="${#want_commit}" HEAD)"

# Resolve the tag we expect and compare it to HEAD, rather than asking
# `git describe` what HEAD happens to be named. actions/checkout clones
# submodules with `git submodule update --depth=1`, and a shallow clone carries
# no tags at all, so `describe` on CI always answers "not on a tag" even when
# the pin is correct. Fetching the single tag we care about takes ~1s and makes
# the check behave the same on CI as in a full local clone.
want_tag="v$want_version"
resolve_tag() { git -C "$core" rev-parse -q --verify "refs/tags/$want_tag^{commit}" || true; }

tag_commit="$(resolve_tag)"
if [ -z "$tag_commit" ]; then
  # --depth=1 only where the clone is already shallow: on a full local checkout
  # it would leave a .git/shallow behind and truncate history nobody asked to
  # lose. Either way this fetches one ref, not the tag list.
  if [ "$(git -C "$core" rev-parse --is-shallow-repository)" = true ]; then
    git -C "$core" fetch --depth=1 --quiet origin "refs/tags/$want_tag:refs/tags/$want_tag" || true
  else
    git -C "$core" fetch --quiet origin "refs/tags/$want_tag:refs/tags/$want_tag" || true
  fi
  tag_commit="$(resolve_tag)"
fi

status=0
if [ -z "$tag_commit" ]; then
  echo "core has no tag $want_tag, locally or on origin, but build.rs CORE_VERSION expects $want_version" >&2
  status=1
elif [ "$tag_commit" != "$head_commit" ]; then
  echo "core submodule is at $got_commit, but tag $want_tag is at $(git -C "$core" rev-parse --short="${#want_commit}" "$tag_commit")" >&2
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
