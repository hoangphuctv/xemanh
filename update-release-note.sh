#!/usr/bin/env bash
# Update GitHub release notes for a tag by reviewing git diff vs a previous tag.
# Usage: ./update-release-note.sh <new-tag> <old-tag>
# Example: ./update-release-note.sh v0.1.18 v0.1.17

set -euo pipefail

normalize_tag() {
  local t="$1"
  if [[ "$t" != v* ]]; then
    t="v$t"
  fi
  printf '%s' "$t"
}

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <new-tag> <old-tag>" >&2
  echo "Example: $0 v0.1.18 v0.1.17" >&2
  exit 1
fi

NEW_TAG="$(normalize_tag "$1")"
OLD_TAG="$(normalize_tag "$2")"
RANGE="${OLD_TAG}..${NEW_TAG}"

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

if ! git rev-parse -q --verify "refs/tags/${NEW_TAG}" >/dev/null; then
  echo "error: tag not found: ${NEW_TAG}" >&2
  exit 1
fi
if ! git rev-parse -q --verify "refs/tags/${OLD_TAG}" >/dev/null; then
  echo "error: tag not found: ${OLD_TAG}" >&2
  exit 1
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "error: gh CLI is required" >&2
  exit 1
fi
if ! command -v agent >/dev/null 2>&1; then
  echo "error: Cursor agent CLI is required (agent on PATH)" >&2
  exit 1
fi

DIFF_EXCLUDES=(-- . ':(exclude)Cargo.lock' ':(exclude)target' ':(exclude).bridge-output')

COMMITS="$(git log "${RANGE}" --reverse --pretty=format:'%h %s')"
DIFF_STAT="$(git diff "${RANGE}" --stat "${DIFF_EXCLUDES[@]}")"
DIFF_PATCH="$(git diff "${RANGE}" "${DIFF_EXCLUDES[@]}")"

if [[ -z "${COMMITS}" && -z "${DIFF_PATCH}" ]]; then
  echo "error: no commits/diff in range ${RANGE}" >&2
  exit 1
fi

MAX_DIFF_CHARS=120000
if ((${#DIFF_PATCH} > MAX_DIFF_CHARS)); then
  echo "warning: git diff is large (${#DIFF_PATCH} chars); truncating for agent prompt" >&2
  DIFF_PATCH="${DIFF_PATCH:0:MAX_DIFF_CHARS}
... [diff truncated for size] ..."
fi

REQUEST_FILE="${ROOT}/.release-notes-request.md"
NOTES_FILE="$(mktemp "${TMPDIR:-/tmp}/xemanh-${NEW_TAG}-notes.XXXXXX")"
if [[ "${NOTES_FILE}" != *.md ]]; then
  mv "${NOTES_FILE}" "${NOTES_FILE}.md"
  NOTES_FILE="${NOTES_FILE}.md"
fi
cleanup() {
  rm -f "${REQUEST_FILE}"
}
trap cleanup EXIT

NEW_VER="${NEW_TAG#v}"

cat >"${REQUEST_FILE}" <<EOF
You are writing GitHub Release notes for XemAnh ${NEW_TAG}.

XemAnh is a lightweight image viewer for Windows & Linux. Readers are end users (not developers).

Range: ${RANGE}

## Commits
${COMMITS}

## git diff --stat
${DIFF_STAT}

## git diff
${DIFF_PATCH}

Write release notes in Vietnamese, clear and friendly:
- ONLY cover end-user facing changes in the XemAnh app itself: viewing images, UI, shortcuts, performance, bug fixes users can notice.
- IGNORE everything that only helps developers or packaging/setup, even if it appears in the diff. Skip: release/build scripts, packaging (.bat/.sh/Inno/deb/dmg), CI, tooling, utils for development, Cargo/deps/lockfile-only changes, README/docs for maintainers, installer plumbing, agent/release-note automation.
- If a commit mixes app behavior and tooling, mention only the app behavior.
- If there are no user-facing app changes, output a single short line: "- Bản cập nhật kỹ thuật (không thay đổi trải nghiệm người dùng)."
- Do NOT invent changes that are not supported by the commits/diff.
- Use GitHub markdown. Prefer sections only when they have items:
  ## Tính năng mới
  ## Cải thiện
  ## Sửa lỗi
- Short bullets; no preamble, no closing remarks, no code fences around the whole note.
- Output ONLY the release notes markdown.
EOF

echo "==> Diff range: ${RANGE}"
echo "==> Asking Cursor agent to draft release notes..."

SHORT_PROMPT="Read the file .release-notes-request.md in the workspace root and follow its instructions exactly."

NOTES_RAW="$(
  agent -p \
    --mode ask \
    --trust \
    --workspace "${ROOT}" \
    --output-format text \
    -- \
    "${SHORT_PROMPT}"
)"

# Strip accidental outer markdown fences
NOTES="$(printf '%s\n' "${NOTES_RAW}" | sed -e '1{/^```\(markdown\|md\)\?$/d;}' -e '${/^```$/d;}')"
NOTES="$(printf '%s\n' "${NOTES}" | sed -e 's/[[:space:]]*$//' | sed -e '/./,$!d')"
# trim trailing blank lines
while [[ "${NOTES}" == *$'\n' ]]; do
  NOTES="${NOTES%$'\n'}"
done

if [[ -z "${NOTES}" ]]; then
  echo "error: agent returned empty release notes" >&2
  exit 1
fi

# Detect classic UTF-8-as-CP437 mojibake (e.g. T├¡nh)
if printf '%s' "${NOTES}" | grep -qE '[├─└┌ß╞╗]'; then
  echo "error: agent output looks encoding-corrupted" >&2
  printf '%s\n' "${NOTES}" >&2
  exit 1
fi

printf '%s\n' "${NOTES}" >"${NOTES_FILE}"

echo ""
echo "==> Release notes written to: ${NOTES_FILE}"
echo "-----"
cat "${NOTES_FILE}"
echo "-----"
echo ""

echo "==> Updating GitHub release ${NEW_TAG} ..."
gh release edit "${NEW_TAG}" --title "${NEW_TAG}" --notes-file "${NOTES_FILE}"

echo "==> Done. Release ${NEW_TAG} notes updated."
echo "Temp notes file kept at: ${NOTES_FILE}"
