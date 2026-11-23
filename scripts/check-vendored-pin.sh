#!/usr/bin/env bash
# Assert the vendored core submodule matches the pin baked into
# rayforce-sys/build.rs.
#
# The build script stamps CORE_VERSION / CORE_COMMIT into librayforce.a because
# a crate unpacked from crates.io has no git history for the core's Makefile to
# read them from (it takes the newest vX.Y.Z tag and `git rev-parse`, see
# Makefile:26 and Makefile:34 in the core). If the constants and the submodule
# disagree, published crates report a version they were not built from.
#
# Two pins pass:
#
#   release pin  The submodule is at tag vCORE_VERSION.
#
#   dev pin      The submodule is at a commit of the core's dev branch, past
#                every release tag. CORE_VERSION is then the newest release
#                that commit contains, which is what the core's Makefile stamps
#                on it. A commit contains a release when the release commit is
#                its ancestor, or when a parent of the release commit with the
#                same tree is: a dev -> master release merge adds no change of
#                its own, and until the core merges it back into dev, only that
#                parent, the release PR's head, is in dev's history. These are
#                the conditions the core's own merge-back checks
#                (.github/release-merge-back.sh there).
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

g() { git -C "$core" "$@"; }

head_commit="$(g rev-parse HEAD)"
got_commit="$(g rev-parse --short="${#want_commit}" HEAD)"
short() { g rev-parse --short="${#want_commit}" "$1"; }

status=0
if [ "$got_commit" != "$want_commit" ]; then
  echo "core submodule is at $got_commit, but build.rs CORE_COMMIT expects $want_commit" >&2
  status=1
fi

# actions/checkout clones submodules with `git submodule update --depth=1`: one
# commit, no tags, no history. So resolve tags by ref rather than asking `git
# describe`, and fetch what a check needs, one ref at a time. Never deepen a
# full local clone (it would leave a .git/shallow behind) or fetch where the
# objects are already here.
is_shallow() { [ "$(g rev-parse --is-shallow-repository)" = true ]; }
resolve_tag() { g rev-parse -q --verify "refs/tags/$1^{commit}" || true; }

# Fetch tag $1 with its parents, which the dev-pin check compares trees with.
fetch_tag() {
  if [ -n "$(resolve_tag "$1")" ] && ! is_shallow; then
    return 0
  fi
  if is_shallow; then
    g fetch --depth=2 --quiet origin "+refs/tags/$1:refs/tags/$1" || true
  else
    g fetch --quiet origin "refs/tags/$1:refs/tags/$1" || true
  fi
}

# Print the commit through which HEAD contains release commit $1, or fail.
contained_via() {
  local release="$1" tree c
  tree="$(g rev-parse "$release^{tree}")"
  for c in "$release" $(g rev-parse "$release^@" 2>/dev/null || true); do
    [ "$(g rev-parse "$c^{tree}" 2>/dev/null || true)" = "$tree" ] || continue
    if g merge-base --is-ancestor "$c" "$head_commit" 2>/dev/null; then
      echo "$c"
      return 0
    fi
  done
  return 1
}

# As contained_via, deepening a shallow clone's history first if that is all
# that stands between HEAD and the release: back to a day before the release
# commit, then, should that not reach it, all of HEAD's history.
contains() {
  contained_via "$1" && return 0
  is_shallow || return 1
  local since
  since="$(g log -1 --format=%ct "$1")"
  g fetch --quiet --shallow-since="$((since - 86400)) +0000" origin "$head_commit" || true
  contained_via "$1" && return 0
  is_shallow || return 1
  g fetch --quiet --unshallow origin "$head_commit" || true
  contained_via "$1"
}

want_tag="v$want_version"
fetch_tag "$want_tag"
tag_commit="$(resolve_tag "$want_tag")"

if [ -z "$tag_commit" ]; then
  echo "core has no tag $want_tag, locally or on origin, but build.rs CORE_VERSION expects $want_version" >&2
  exit 1
fi

if [ "$tag_commit" = "$head_commit" ]; then
  if [ "$status" -eq 0 ]; then
    echo "vendored core pin OK: release v$want_version ($want_commit)"
  fi
  exit "$status"
fi

# Not the release itself, so a dev pin: it must contain v$want_version ...
if ! via="$(contains "$tag_commit")"; then
  echo "core submodule is at $got_commit, which is neither tag $want_tag ($(short "$tag_commit")) nor a commit that contains it" >&2
  exit 1
fi

# ... and no newer release, or CORE_VERSION would understate what it carries.
# Only tags past CORE_VERSION are checked, and a release cut later from a
# newer dev is not contained, so a dev pin that passes keeps passing.
if ! remote_tags="$(g ls-remote --tags --refs origin 'v*')"; then
  echo "could not list the core's tags on origin, so the dev pin's newest release is unchecked" >&2
  exit 1
fi
newer="$(printf '%s\n' "$remote_tags" |
  sed -n 's|.*refs/tags/\(v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$|\1|p' |
  { cat; echo "$want_tag"; } | sort -V -u |
  awk -v want="$want_tag" 'past { print } $0 == want { past = 1 }')"
for t in $newer; do
  fetch_tag "$t"
  c="$(resolve_tag "$t")"
  if [ -n "$c" ] && contains "$c" >/dev/null; then
    echo "core submodule $got_commit contains $t, so build.rs CORE_VERSION should be ${t#v}, not $want_version" >&2
    status=1
  fi
done

if [ "$status" -eq 0 ]; then
  if [ "$via" = "$tag_commit" ]; then
    how="its ancestor"
  else
    how="via $(short "$via"), its parent with the same tree"
  fi
  echo "vendored core pin OK: dev $want_commit, newest release it contains v$want_version ($(short "$tag_commit"), $how)"
fi
exit "$status"
