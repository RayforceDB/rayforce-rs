#!/bin/bash

# Usage: ./announce-release.sh VERSION [BOT_API_KEY] [DEBUG]
#
# Extracts the section for VERSION from the changelog and posts it to Zulip.
# Pass DEBUG=1 as the third argument to print the message instead of sending it.

VERSION=${1}
BOT_API_KEY=${2}
DEBUG=${3}

if [ -z "$VERSION" ]; then
  echo "Error: VERSION is required"
  echo "Usage: $0 VERSION [BOT_API_KEY]"
  exit 1
fi

if [ -z "$BOT_API_KEY" ] && [ "$DEBUG" != 1 ]; then
  echo "Error: BOT_API_KEY is required"
  echo "Usage: $0 VERSION BOT_API_KEY"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CHANGELOG="${PROJECT_ROOT}/docs/docs/content/CHANGELOG.md"

if [ ! -f "${CHANGELOG}" ]; then
  echo "Warning: CHANGELOG not found at ${CHANGELOG}"
  CHANGELOG_CONTENT=""
else
  # Collect everything under `## VERSION` up to (but not including) the next
  # `## ` heading. Headings look like `## 0.1.0`.
  CHANGELOG_CONTENT=$(awk -v version="${VERSION}" '
    BEGIN {
      collecting=0
      found_version=0
      escaped_version = version
      gsub(/\./, "\\.", escaped_version)
    }
    {
      # Stop at the next version heading (check before collecting it).
      if (collecting && $0 ~ /^## /) {
        exit 0
      }
      # Start collecting at `## VERSION` (allow optional trailing text).
      if ($0 ~ "^## " escaped_version "([^0-9].*)?$") {
        collecting=1
        found_version=1
        print
        next
      }
      if (collecting) {
        print
      }
    }
    END {
      if (!found_version) {
        exit 1
      }
    }
  ' "${CHANGELOG}")

  if [ $? -ne 0 ] || [ -z "${CHANGELOG_CONTENT}" ]; then
    echo "Exit: No changelog entry found for version ${VERSION}"
    exit 1
  fi
fi

# Pre-release label for SemVer alpha/beta/rc versions.
LABEL=""
case "${VERSION}" in
  *-alpha.*) LABEL="[ALPHA] " ;;
  *-beta.*)  LABEL="[BETA] "  ;;
  *-rc.*)    LABEL="[RC] "    ;;
esac

CONTENT="**${LABEL}New Rayforce-Rs Version is Released!**

**[🔗 crates.io](https://crates.io/crates/rayforce/${VERSION})** | **[🔗 docs.rs](https://docs.rs/rayforce/${VERSION})** | **[🔗 GitHub](https://github.com/RayforceDB/rayforce-rs/releases/tag/${VERSION})**"

if [ -n "${CHANGELOG_CONTENT}" ]; then
  CONTENT="${CONTENT}

${CHANGELOG_CONTENT}"
fi

if [ "$DEBUG" == 1 ]; then
  echo "${CONTENT}"
  exit 0
fi

curl -X POST https://rayforcedb.zulipchat.com/api/v1/messages \
  -u releases-bot@rayforcedb.zulipchat.com:${BOT_API_KEY} \
  -d type=stream \
  -d "to=Announcements" \
  -d topic="Rayforce-Rs" \
  -d "content=${CONTENT}"

echo ""
echo "✅ Announcement sent to Zulip!"
