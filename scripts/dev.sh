#!/usr/bin/env bash
#
# Build SingSin, run the local CORS relay, serve the bundle and open it in
# your browser.
#
# Usage:
#   ./scripts/dev.sh                 build + proxy + serve + open the browser
#   ./scripts/dev.sh --no-build      reuse the existing dist/
#   ./scripts/dev.sh --no-open       don't open a browser
#   ./scripts/dev.sh --port 9000     serve on a different port
#   ./scripts/dev.sh --relay-port 9001
#
# Press Ctrl-C to stop everything.
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PORT=8099
RELAY_PORT=8081
DO_BUILD=1
DO_OPEN=1

while [ $# -gt 0 ]; do
  case "$1" in
    --no-build)     DO_BUILD=0; shift ;;
    --no-open)      DO_OPEN=0; shift ;;
    --port)         PORT="${2:?--port needs a number}"; shift 2 ;;
    --port=*)       PORT="${1#*=}"; shift ;;
    --relay-port)   RELAY_PORT="${2:?--relay-port needs a number}"; shift 2 ;;
    --relay-port=*) RELAY_PORT="${1#*=}"; shift ;;
    -h|--help)      awk 'NR>1 && /^#/ {sub(/^# ?/,""); print; next} NR>1 {exit}' "$0"; exit 0 ;;
    *) echo "unknown flag: $1 (try --help)" >&2; exit 2 ;;
  esac
done

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  BOLD=$'\033[1m'; DIM=$'\033[2m'; GREEN=$'\033[32m'; RED=$'\033[31m'; YELLOW=$'\033[33m'; OFF=$'\033[0m'
else
  BOLD=""; DIM=""; GREEN=""; RED=""; YELLOW=""; OFF=""
fi

step() { printf '\n%s==>%s %s%s%s\n' "$BOLD" "$OFF" "$BOLD" "$1" "$OFF"; }
info() { printf '    %s%s%s\n' "$DIM" "$1" "$OFF"; }
good() { printf '    %s✓%s %s\n' "$GREEN" "$OFF" "$1"; }
warn() { printf '    %s!%s %s\n' "$YELLOW" "$OFF" "$1"; }
die()  { printf '\n%serror:%s %s\n' "$RED" "$OFF" "$1" >&2; exit 1; }

WORK="$(mktemp -d)"
RELAY_PID=""
SERVER_PID=""

cleanup() {
  trap - INT TERM EXIT
  [ -n "$RELAY_PID" ] && kill "$RELAY_PID" 2>/dev/null || true
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  echo
  printf '%sstopped.%s\n' "$BOLD" "$OFF"
}
trap cleanup INT TERM EXIT

# Is something already listening on this port?
port_busy() {
  python3 - "$1" <<'PY'
import socket, sys
s = socket.socket()
s.settimeout(0.5)
try:
    s.connect(("127.0.0.1", int(sys.argv[1])))
    s.close()
    sys.exit(0)
except OSError:
    sys.exit(1)
PY
}

wait_for() {
  local url="$1" name="$2"
  for _ in $(seq 1 60); do
    if curl -sS -o /dev/null --max-time 1 "$url" 2>/dev/null; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

open_browser() {
  local url="$1"
  if command -v open >/dev/null 2>&1; then
    open "$url"
  elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$url"
  else
    return 1
  fi
}

# ----------------------------------------------------------------- build ---

step "Build"
if [ "$DO_BUILD" -eq 1 ]; then
  command -v trunk >/dev/null 2>&1 || die "trunk is not installed (cargo install trunk)"
  trunk build --release
  good "dist/ is up to date"
else
  warn "skipped (--no-build), using the existing dist/"
fi
[ -f dist/index.html ] || die "dist/index.html is missing — run without --no-build"

# ----------------------------------------------------------------- relay ---

step "CORS relay (words.hk)"
if port_busy "$RELAY_PORT"; then
  warn "port $RELAY_PORT is already in use — reusing the relay that is already running"
else
  python3 tools/proxy.py --host 127.0.0.1 --port "$RELAY_PORT" > "$WORK/relay.log" 2>&1 &
  RELAY_PID=$!
  if wait_for "http://127.0.0.1:$RELAY_PORT/?url=https%3A%2F%2Fwords.hk%2Fstatic%2Fjyutping%2Fnei5.mp3" relay; then
    good "relay on http://127.0.0.1:$RELAY_PORT (pid $RELAY_PID)"
  else
    cat "$WORK/relay.log" >&2
    die "the relay did not come up on port $RELAY_PORT"
  fi
fi

# ---------------------------------------------------------------- server ---

step "Static server"
SITE_URL="http://127.0.0.1:$PORT/index.html"
if port_busy "$PORT"; then
  warn "port $PORT is already in use — reusing the server that is already running"
else
  python3 -m http.server "$PORT" --bind 127.0.0.1 --directory dist > "$WORK/server.log" 2>&1 &
  SERVER_PID=$!
  if wait_for "$SITE_URL" server; then
    good "serving dist/ on http://127.0.0.1:$PORT (pid $SERVER_PID)"
  else
    cat "$WORK/server.log" >&2
    die "the static server did not come up on port $PORT"
  fi
fi

# ---------------------------------------------------------------- browse ---

step "Open"
if [ "$DO_OPEN" -eq 1 ]; then
  if open_browser "$SITE_URL"; then
    good "opened $SITE_URL"
  else
    warn "could not open a browser automatically — visit $SITE_URL"
  fi
else
  info "$SITE_URL"
fi

printf '\n%s%sSingSin is running.%s\n' "$BOLD" "$GREEN" "$OFF"
printf '    site    %s\n' "$SITE_URL"
printf '    relay   http://127.0.0.1:%s/?url=<encoded>\n' "$RELAY_PORT"
printf '    %sCtrl-C to stop.%s\n' "$DIM" "$OFF"

# Keep the servers alive until the user interrupts. If we reused servers that
# were already running there are no child jobs to wait on, so sleep instead.
if [ -n "$RELAY_PID" ] || [ -n "$SERVER_PID" ]; then
  wait
else
  while true; do sleep 3600; done
fi
