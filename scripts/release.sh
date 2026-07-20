#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# --- Files containing version strings ---
# The workspace version is the single source of truth: every crate inherits it
# via `version.workspace = true`. The `[workspace.dependencies]` entry for
# `rayforce-sys` also pins the same version and must move in lockstep.
CARGO_TOML="${PROJECT_ROOT}/Cargo.toml"

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# --- Helpers ---
info()  { echo -e "${CYAN}${BOLD}::${NC} $1"; }
ok()    { echo -e "${GREEN}${BOLD}✓${NC}  $1"; }
warn()  { echo -e "${YELLOW}${BOLD}!${NC}  $1"; }
err()   { echo -e "${RED}${BOLD}✗${NC}  $1" >&2; }
die()   { err "$1"; exit 1; }

# --- Validate input ---
VERSION="$1"

if [ -z "$VERSION" ]; then
  echo ""
  echo -e "  ${BOLD}Usage:${NC} ./scripts/release.sh <version>"
  echo ""
  echo "  Examples:"
  echo "    ./scripts/release.sh 0.1.1          # stable"
  echo "    ./scripts/release.sh 0.2.0-alpha.1  # alpha pre-release"
  echo "    ./scripts/release.sh 0.2.0-beta.1   # beta"
  echo "    ./scripts/release.sh 0.2.0-rc.1     # release candidate"
  echo ""
  exit 1
fi

# SemVer (final or pre-release): X.Y.Z optionally followed by -alpha.N, -beta.N,
# or -rc.N. Matches the Cargo.toml version and the git tag we create.
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-(alpha|beta|rc)\.[0-9]+)?$'; then
  die "Invalid version format: '${VERSION}' (expected X.Y.Z[-alpha.N|-beta.N|-rc.N])"
fi

# --- Pre-flight checks ---
cd "$PROJECT_ROOT"

# First `version = "X.Y.Z"` under [workspace.package].
CURRENT_VERSION=$(grep -m1 '^version = ' "$CARGO_TOML" | sed 's/.*"\(.*\)".*/\1/')
info "Current version: ${BOLD}${CURRENT_VERSION}${NC}"
info "New version:     ${BOLD}${VERSION}${NC}"
echo ""

if [ "$CURRENT_VERSION" = "$VERSION" ]; then
  die "Version ${VERSION} is already the current version"
fi

if ! command -v cargo >/dev/null 2>&1; then
  die "cargo not found on PATH"
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  die "Working tree is dirty. Commit or stash changes first."
fi

if git rev-parse "$VERSION" >/dev/null 2>&1; then
  die "Git tag '${VERSION}' already exists"
fi

# --- Update version in Cargo.toml ---
info "Updating version strings..."

# [workspace.package] version = "X.Y.Z"
sed -i '' "s/^version = \"${CURRENT_VERSION}\"/version = \"${VERSION}\"/" "$CARGO_TOML"
# [workspace.dependencies] rayforce-sys = { path = "rayforce-sys", version = "X.Y.Z" }
sed -i '' "s/\(rayforce-sys = { path = \"rayforce-sys\", version = \"\)${CURRENT_VERSION}\(\" }\)/\1${VERSION}\2/" "$CARGO_TOML"
ok "Cargo.toml"

# Refresh Cargo.lock so the workspace crates pick up the new version.
if [ -f "${PROJECT_ROOT}/Cargo.lock" ]; then
  cargo update --workspace --offline >/dev/null 2>&1 || cargo update --workspace >/dev/null 2>&1 || true
  ok "Cargo.lock"
fi

echo ""

# --- Summary ---
info "Changes to be committed:"
echo ""
git diff --stat
echo ""

read -r -p "$(echo -e "${CYAN}${BOLD}::${NC} Commit and tag as ${BOLD}${VERSION}${NC}? [Y/n] ")" CONFIRM
CONFIRM=${CONFIRM:-y}

if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
  warn "Aborted. Version files have been updated but not committed."
  warn "Run 'git checkout -- .' to revert."
  exit 1
fi

# --- Git commit & tag ---
git add "$CARGO_TOML"
[ -f "${PROJECT_ROOT}/Cargo.lock" ] && git add "${PROJECT_ROOT}/Cargo.lock"
git commit -m "release: ${VERSION}"
git tag "$VERSION"

ok "Committed and tagged ${BOLD}${VERSION}${NC}"
echo ""

# --- Push ---
read -r -p "$(echo -e "${CYAN}${BOLD}::${NC} Push to origin? [Y/n] ")" PUSH
PUSH=${PUSH:-y}

if [[ "$PUSH" =~ ^[Yy]$ ]]; then
  git push && git push --tags
  ok "Pushed to origin — the release workflow will publish to crates.io"
else
  echo ""
  warn "Remember to push manually:"
  echo "  git push && git push --tags"
fi

echo ""
echo -e "${GREEN}${BOLD}Release ${VERSION} complete!${NC}"
