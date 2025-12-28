#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: scripts/release_local.sh <version>

Bumps versions in:
  - apps/desktop/src-tauri/Cargo.toml
  - CHANGELOG.md (moves [Unreleased] into a new release section)

Then builds:
  - web (npm ci + npm run build)
  - desktop bundles (cargo tauri build)

Example:
  bash scripts/release_local.sh 0.2.0
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -ne 1 ]]; then
  usage
  exit 2
fi

VERSION="${1#v}"

# SemVer-ish validation (accepts 0.x.y; disallows leading zeros in numeric identifiers)
SEMVER_REGEX='^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$'
if [[ ! "${VERSION}" =~ ${SEMVER_REGEX} ]]; then
  echo "Invalid version: '${1}' (expected SemVer like 0.2.0, 1.2.3-rc.1, or 1.2.3+build.5)" >&2
  exit 2
fi

RELEASE_DATE="$(date +%F)"

update_cargo_toml_version() {
  local file="$1"
  perl -0777 -pi -e 's/(\[package\][\s\S]*?^version\s*=\s*")([^"]*)(")/$1$ENV{RELEASE_VERSION}$3/m or die "Failed to update version in $ARGV\n";' "${file}"
}

update_changelog() {
  local file="$1"
  perl -0777 -pi -e '
    my $ver = $ENV{RELEASE_VERSION};
    my $date = $ENV{RELEASE_DATE};

    die "CHANGELOG.md already contains release section for $ver\n"
      if $_ =~ /^\Q## [$ver]\E/m;

    my $marker = "## [Unreleased]\n";
    my $idx = index($_, $marker);
    die "CHANGELOG.md is missing an [Unreleased] section\n" if $idx < 0;

    my $before = substr($_, 0, $idx + length($marker));
    my $rest = substr($_, $idx + length($marker));

    my $next = -1;
    if ($rest =~ /(^## \[)/m) {
      $next = $-[0];
    }

    my $unreleased_body = $next >= 0 ? substr($rest, 0, $next) : $rest;
    my $after = $next >= 0 ? substr($rest, $next) : "";

    my $release_body = $unreleased_body;
    $release_body =~ s/\A\n+//s;
    $release_body =~ s/\n+\z/\n/s;

    my $new_unreleased =
      "\n".
      "### Added\n\n".
      "### Changed\n\n".
      "### Fixed\n\n".
      "### Removed\n\n";

    my $new_release =
      "## [$ver] - $date\n\n".
      ($release_body =~ /\S/ ? $release_body : "- Release $ver.\n") .
      "\n";

    $_ = $before . $new_unreleased . $new_release . $after;
  ' "${file}"
}

export RELEASE_VERSION="${VERSION}"
export RELEASE_DATE

update_cargo_toml_version "${ROOT_DIR}/apps/desktop/src-tauri/Cargo.toml"
update_changelog "${ROOT_DIR}/CHANGELOG.md"

echo "Version bumped to ${VERSION} (${RELEASE_DATE})."

cd "${ROOT_DIR}/apps/web"
npm ci
npm run build

cd "${ROOT_DIR}/apps/desktop/src-tauri"
cargo tauri build

echo "Local release build complete."
echo "Artifacts are unsigned/not notarized. See docs/release.md."
