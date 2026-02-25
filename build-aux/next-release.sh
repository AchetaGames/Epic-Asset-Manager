#!/bin/bash
# Prepare a new release: bump version, generate changelog, commit, and tag.
#
# Usage:
#   ./build-aux/next-release.sh major|minor|patch [OPTIONS]
#
# Options:
#   --dry-run       Show what would be done without modifying anything
#   --push          Push the commit and tag to origin after creating them
#   --message MSG   Custom commit message (default: "Release vX.Y.Z")
#   --help          Show this help message

set -euo pipefail

# ── Helpers ──────────────────────────────────────────────────────────────────

die()  { echo "error: $*" >&2; exit 1; }
info() { echo "==> $*"; }

usage() {
    sed -n '2,/^$/{ s/^# \?//; p }' "$0"
    exit 0
}

# ── Parse arguments ──────────────────────────────────────────────────────────

DRY_RUN=false
PUSH=false
COMMIT_MSG=""
BUMP=""

while [[ $# -gt 0 ]]; do
    case $1 in
        major|minor|patch) BUMP="$1" ;;
        --dry-run)         DRY_RUN=true ;;
        --push)            PUSH=true ;;
        --message)         shift; COMMIT_MSG="${1:?--message requires a value}" ;;
        --help|-h)         usage ;;
        *)                 die "unknown argument: $1 (see --help)" ;;
    esac
    shift
done

[[ -n "$BUMP" ]] || die "bump type required: major, minor, or patch (see --help)"

# ── Read current version ─────────────────────────────────────────────────────

cd "$(git rev-parse --show-toplevel)" || die "not inside a git repository"

current=$(grep -Po "version: '\K([0-9]+\.[0-9]+\.[0-9]+)(?=')" meson.build) \
    || die "could not read version from meson.build"
id=$(grep -Po "base_id\s+=\s+'\K[^']*" meson.build) \
    || die "could not read base_id from meson.build"

IFS='.' read -r major minor patch <<< "$current"

case $BUMP in
    major) next="$((major + 1)).0.0" ;;
    minor) next="${major}.$((minor + 1)).0" ;;
    patch) next="${major}.${minor}.$((patch + 1))" ;;
esac

COMMIT_MSG="${COMMIT_MSG:-Release v${next}}"
METAINFO="data/${id}.metainfo.xml.in.in"

[[ -f "$METAINFO" ]] || die "metainfo not found: $METAINFO"

# ── Pre-flight checks ───────────────────────────────────────────────────────

if [[ "$DRY_RUN" == false ]]; then
    if ! git diff --quiet || ! git diff --cached --quiet; then
        die "working tree is dirty — commit or stash changes first"
    fi
fi

if ! git describe --tags --abbrev=0 &>/dev/null; then
    echo "warning: no previous tag found — changelog will include all commits" >&2
fi

# ── Generate changelog from git log ─────────────────────────────────────────

prev_tag=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
if [[ -n "$prev_tag" ]]; then
    range="${prev_tag}..HEAD"
else
    range="HEAD"
fi

changelog_items=""
while IFS= read -r subject; do
    [[ -z "$subject" ]] && continue
    changelog_items+="                    <li>${subject}</li>"$'\n'
done < <(git log "$range" --no-merges --format="%s")

# Trim trailing newline
changelog_items="${changelog_items%$'\n'}"

today=$(date +%F)

# Build the release XML block
release_block="    <release version=\"${next}\" date=\"${today}\">
        <description>
            <p>Release ${next}</p>
            <ul>
${changelog_items}
            </ul>
        </description>
    </release>"

# ── Dry-run output ───────────────────────────────────────────────────────────

if [[ "$DRY_RUN" == true ]]; then
    info "DRY RUN — no files will be modified"
    echo ""
    echo "Version bump: ${current} → ${next} (${BUMP})"
    echo "Commit message: ${COMMIT_MSG}"
    echo ""
    echo "Files to update:"
    echo "  meson.build   version: '${current}' → '${next}'"
    echo "  Cargo.toml    version = \"${current}\" → \"${next}\""
    echo "  ${METAINFO}"
    echo ""
    echo "Changelog entries (${range}):"
    if [[ -n "$changelog_items" ]]; then
        echo "$release_block"
    else
        echo "  (no commits found)"
    fi
    echo ""
    if [[ "$PUSH" == true ]]; then
        echo "Would push: commit + tag v${next} to origin"
    else
        echo "Would NOT push (use --push to push after commit+tag)"
    fi
    exit 0
fi

# ── Apply changes ────────────────────────────────────────────────────────────

info "Bumping version: ${current} → ${next}"

sed -i "s/version: '${current}'/version: '${next}'/" meson.build
sed -i "s/version = \"${current}\"/version = \"${next}\"/" Cargo.toml

info "Updating ${METAINFO}"

# Insert the new release block after <releases>
sed -i "/<releases>/a\\
${release_block}" "$METAINFO"

# ── Commit & tag ─────────────────────────────────────────────────────────────

info "Committing: ${COMMIT_MSG}"

git add meson.build Cargo.toml "$METAINFO"
git commit -m "$COMMIT_MSG"
git tag "v${next}"

info "Tagged v${next}"

# ── Push (opt-in) ────────────────────────────────────────────────────────────

if [[ "$PUSH" == true ]]; then
    info "Pushing to origin..."
    git push origin HEAD
    git push origin "v${next}"
    info "Done — pushed commit and tag v${next}"
else
    echo ""
    echo "Release v${next} prepared locally. Next steps:"
    echo "  git push origin HEAD"
    echo "  git push origin v${next}"
fi
