#!/usr/bin/env bash
# Black-box smoke test against a running server (local, Docker, or deployed).
#
#   scripts/smoke.sh [base_url]            # offline checks only
#   SMOKE_LIVE=1 scripts/smoke.sh          # also search, queue and download a
#                                          # real chapter from SMOKE_SOURCE
#
# Offline checks need no upstream site: health, source catalogue, OpenAPI,
# and request validation. The live pass hits the real source, so it belongs in
# the nightly workflow, not in per-push CI.
#
# Needs curl, jq and unzip.

set -euo pipefail

BASE="${1:-${SMOKE_BASE_URL:-http://localhost:8000}}"
BASE="${BASE%/}"
SOURCE="${SMOKE_SOURCE:-mangapill}"
QUERY="${SMOKE_QUERY:-solo leveling}"
FORMAT="${SMOKE_FORMAT:-cbz}"
TIMEOUT="${SMOKE_TIMEOUT:-300}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

pass() { printf '  \033[32mok\033[0m   %s\n' "$1"; }
fail() { printf '  \033[31mFAIL\033[0m %s\n' "$1" >&2; exit 1; }

# status_of METHOD PATH [JSON_BODY] -> prints the HTTP status, body in $WORK/body
status_of() {
  local args=(-s -o "$WORK/body" -w '%{http_code}' -X "$1" "$BASE$2")
  if [[ $# -ge 3 ]]; then
    args+=(-H 'content-type: application/json' --data "$3")
  fi
  curl "${args[@]}"
}

expect() { # expect WANT METHOD PATH [BODY] -- label
  local want="$1"; shift
  local label="${*: -1}"
  local got
  got="$(status_of "${@:1:$#-1}")"
  [[ "$got" == "$want" ]] || fail "$label: wanted HTTP $want, got $got: $(head -c 300 "$WORK/body")"
  pass "$label ($got)"
}

wait_healthy() {
  local deadline=$((SECONDS + 60))
  until curl -sf "$BASE/health" -o /dev/null; do
    (( SECONDS < deadline )) || fail "server at $BASE never became healthy"
    sleep 1
  done
}

echo "smoke: $BASE"
wait_healthy

expect 200 GET /health "health"
jq -e '.status == "ok" and (.capabilities.formats | index("pdf"))' "$WORK/body" >/dev/null \
  || fail "health body: $(cat "$WORK/body")"

expect 200 GET /sources "source catalogue"
jq -e 'length >= 8' "$WORK/body" >/dev/null || fail "fewer than 8 sources listed"

expect 200 GET /openapi.json "openapi document"
jq -e '.paths["/download"].post' "$WORK/body" >/dev/null || fail "openapi lacks POST /download"

expect 400 POST /download '' "empty download body rejected"
expect 400 POST /download '{"source":"mangadex","chapters":[]}' "no chapters rejected"
expect 400 POST /download '{"source":"nope","chapters":[{"id":"1","number":"1"}]}' "unknown source rejected"
expect 404 GET /download/status/00000000-0000-4000-8000-000000000000 "unknown task is 404"

if [[ "${SMOKE_LIVE:-0}" != "1" ]]; then
  echo "smoke: offline checks passed (set SMOKE_LIVE=1 for a real download)"
  exit 0
fi

echo "smoke: live download from $SOURCE"
q="$(jq -rn --arg q "$QUERY" '$q|@uri')"
expect 200 GET "/search?q=$q&source=$SOURCE" "search $SOURCE"
jq -r --arg s "$SOURCE" '.results[$s][]?.id' "$WORK/body" | head -n 5 > "$WORK/comics"
[[ -s "$WORK/comics" ]] || fail "no search results: $(jq -c .errors "$WORK/body")"

# Licensed titles often list no chapters, so take the first hit that has one.
chapter=""
while read -r comic; do
  c="$(jq -rn --arg c "$comic" '$c|@uri')"
  [[ "$(status_of GET "/chapters?source=$SOURCE&id=$c")" == 200 ]] || continue
  chapter="$(jq -c '[.volumes[].chapters[]][0] // empty | {id, number}' "$WORK/body")"
  [[ -n "$chapter" ]] && break
done < "$WORK/comics"
[[ -n "$chapter" ]] || fail "none of the first 5 results for '$QUERY' has chapters"
pass "chapters for $comic"

body="$(jq -cn --arg s "$SOURCE" --arg f "$FORMAT" --argjson ch "$chapter" \
  '{source:$s, comic_title:"smoke", format:$f, chapters:[$ch]}')"
expect 202 POST /download "$body" "queue one chapter"
task="$(jq -r .task_id "$WORK/body")"

deadline=$((SECONDS + TIMEOUT))
while :; do
  status_of GET "/download/status/$task" >/dev/null
  state="$(jq -r .state "$WORK/body")"
  case "$state" in
    SUCCESS) break ;;
    FAILURE|CANCELLED) fail "task $task ended $state: $(jq -c . "$WORK/body")" ;;
  esac
  (( SECONDS < deadline )) || fail "task $task still $state after ${TIMEOUT}s"
  sleep 2
done
size="$(jq -r .file_size "$WORK/body")"
pass "task finished ($size bytes)"

code="$(curl -s -o "$WORK/archive.zip" -w '%{http_code}' "$BASE/download/file/$task")"
[[ "$code" == 200 ]] || fail "file fetch returned HTTP $code"
[[ "$(wc -c < "$WORK/archive.zip" | tr -d ' ')" == "$size" ]] || fail "archive size differs from file_size"
unzip -tq "$WORK/archive.zip" >/dev/null || fail "archive is not a valid zip"
pass "archive downloaded and verified"

jq -e '.warnings | not' "$WORK/body" >/dev/null || echo "  note: task reported warnings: $(jq -c .warnings "$WORK/body")"
echo "smoke: live download passed"
