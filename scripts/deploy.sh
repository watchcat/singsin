#!/usr/bin/env bash
#
# Build SingSin and deploy it to Cloudflare.
#
# What it does, in order:
#   1. runs the test suite
#   2. builds the release bundle with Trunk      -> dist/
#   3. uploads every asset to the R2 bucket       -> singsin-site
#   4. prunes assets left over from older builds
#   5. uploads the app Worker + the relay Worker  -> singsin-app, singsin-relay
#   6. enables the workers.dev subdomain for both
#   7. verifies the public URLs answer
#
# Usage:
#   ./scripts/deploy.sh                build + deploy everything
#   ./scripts/deploy.sh --dry-run      build, then print the plan (no writes)
#   ./scripts/deploy.sh --no-build     deploy the existing dist/ as-is
#   ./scripts/deploy.sh --skip-tests   skip the test suite
#
# Credentials (required):
#   export CLOUDFLARE_API_TOKEN=...        # API token with Workers + R2 edit
#
# Optional overrides:
#   CLOUDFLARE_ACCOUNT_ID   (default: the account this project was set up in)
#   CLOUDFLARE_BUCKET       (default: singsin-site)
#   CLOUDFLARE_APP_WORKER   (default: singsin-app)
#   CLOUDFLARE_RELAY_WORKER (default: singsin-relay)
#   CLOUDFLARE_SUBDOMAIN    (default: watchcat)
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

API="${CLOUDFLARE_API_URL:-https://api.cloudflare.com/client/v4}"
CF_ACCOUNT="${CLOUDFLARE_ACCOUNT_ID:-6c27fb90376881b1dd83114b986f17ab}"
BUCKET="${CLOUDFLARE_BUCKET:-singsin-site}"
APP_WORKER="${CLOUDFLARE_APP_WORKER:-singsin-app}"
RELAY_WORKER="${CLOUDFLARE_RELAY_WORKER:-singsin-relay}"
SUBDOMAIN="${CLOUDFLARE_SUBDOMAIN:-watchcat}"
COMPAT_DATE="${CLOUDFLARE_COMPAT_DATE:-2026-11-04}"

DRY_RUN=0
SKIP_BUILD=0
SKIP_TESTS=0

for arg in "$@"; do
  case "$arg" in
    --dry-run)    DRY_RUN=1 ;;
    --no-build)   SKIP_BUILD=1 ;;
    --skip-tests) SKIP_TESTS=1 ;;
    -h|--help)    awk 'NR>1 && /^#/ {sub(/^# ?/,""); print; next} NR>1 {exit}' "$0"; exit 0 ;;
    *) echo "unknown flag: $arg (try --help)" >&2; exit 2 ;;
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
cleanup() { [ -n "${RELAY_PID:-}" ] && kill "$RELAY_PID" 2>/dev/null || true; }
trap cleanup EXIT

# ---------------------------------------------------------------- helpers ---

api() {
  local method="$1" path="$2"; shift 2
  curl -sS --fail-with-body -X "$method" "$API$path" \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
    -H "Content-Type: application/json" \
    "$@" > "$WORK/resp.json"
}

# Fail loudly unless the last response was a Cloudflare `success: true`.
ok_or_die() {
  local ctx="$1"
  FILE="$WORK/resp.json" CTX="$ctx" python3 - <<'PY'
import json, os, sys
raw = open(os.environ["FILE"]).read()
try:
    d = json.loads(raw)
except Exception:
    sys.exit(f"{os.environ['CTX']}: response was not JSON: {raw[:400]}")
if not d.get("success", False):
    errs = "; ".join(f"{e.get('code')}: {e.get('message')}" for e in d.get("errors", []))
    sys.exit(f"{os.environ['CTX']}: Cloudflare API error -> {errs or raw[:400]}")
PY
}

ctype_for() {
  case "$1" in
    *.html) echo "text/html; charset=utf-8" ;;
    *.js)   echo "text/javascript; charset=utf-8" ;;
    *.wasm) echo "application/wasm" ;;
    *.css)  echo "text/css; charset=utf-8" ;;
    *.json) echo "application/json" ;;
    *.svg)  echo "image/svg+xml" ;;
    *.png)  echo "image/png" ;;
    *)      echo "application/octet-stream" ;;
  esac
}

urlenc() { python3 -c 'import sys,urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$1"; }

# ------------------------------------------------------------- preflight ---

step "Preflight"
command -v trunk   >/dev/null 2>&1 || die "trunk is not installed (cargo install trunk)"
command -v curl    >/dev/null 2>&1 || die "curl is not installed"
command -v python3 >/dev/null 2>&1 || die "python3 is not installed"
if [ "$DRY_RUN" -eq 1 ]; then
  warn "dry run — nothing will be written to Cloudflare"
else
  [ -n "${CLOUDFLARE_API_TOKEN:-}" ] || die "CLOUDFLARE_API_TOKEN is not set. Create an API token with Workers + R2 edit permissions and export it."
fi
info "account  $CF_ACCOUNT"
info "api      $API"
info "bucket   $BUCKET"
info "workers  $APP_WORKER, $RELAY_WORKER"

# ------------------------------------------------------------------ test ---

if [ "$SKIP_TESTS" -eq 0 ]; then
  step "Tests"
  cargo test --test phonetics
  good "phonetic engine, Jyutping parser and DSP all pass"
fi

# ----------------------------------------------------------------- build ---

if [ "$SKIP_BUILD" -eq 0 ]; then
  step "Build"
  trunk build --release
  good "dist/ is up to date"
else
  step "Build"
  warn "skipped (--no-build), using the existing dist/"
fi

shopt -s nullglob
# index.html goes last on purpose: it is the pointer to the hashed assets, so
# it must only flip once every asset it references is already in R2.
assets=(dist/*.js dist/*.wasm dist/*.css dist/index.html)
shopt -u nullglob
[ "${#assets[@]}" -gt 0 ] || die "dist/ has no index.html — run without --no-build"

[ "${#assets[@]}" -ge 4 ] || warn "only ${#assets[@]} asset(s) found in dist/"

keys=()
for f in "${assets[@]}"; do
  keys+=("$(basename "$f")")
done

step "Assets"
for f in "${assets[@]}"; do
  k="$(basename "$f")"
  printf '    %-44s %8s B  %s\n' "$k" "$(wc -c < "$f" | tr -d ' ')" "$(ctype_for "$k")"
done

# ----------------------------------------------------------------- bucket ---

step "R2 bucket"
if [ "$DRY_RUN" -eq 1 ]; then
  warn "would ensure bucket $BUCKET exists"
else
  if api GET "/accounts/$CF_ACCOUNT/r2/buckets/$BUCKET"; then
    ok_or_die "get bucket"
    good "bucket $BUCKET already exists"
  else
    api PUT "/accounts/$CF_ACCOUNT/r2/buckets/$BUCKET" -X PUT --data '{}'
    ok_or_die "create bucket"
    good "created bucket $BUCKET"
  fi
fi

# ---------------------------------------------------------------- uploads ---

step "Upload assets"
if [ "$DRY_RUN" -eq 1 ]; then
  warn "would upload ${#assets[@]} object(s) to r2://$BUCKET"
else
  for f in "${assets[@]}"; do
    k="$(basename "$f")"
    ct="$(ctype_for "$k")"
    curl -sS --fail-with-body -X PUT "$API/accounts/$CF_ACCOUNT/r2/buckets/$BUCKET/objects/$(urlenc "$k")" \
      -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
      -H "Content-Type: $ct" \
      --data-binary "@$f" > "$WORK/resp.json"
    ok_or_die "upload $k"
    printf '    %-44s %s\n' "$k" "$ct"
  done
  good "uploaded ${#assets[@]} object(s)"
fi

# ------------------------------------------------------------------ prune ---

step "Prune stale assets"
if [ "$DRY_RUN" -eq 1 ]; then
  warn "would list r2://$BUCKET and delete anything not in the current build"
else
  printf '%s\n' "${keys[@]}" > "$WORK/keep.txt"
  curl -sS --fail-with-body -X GET \
    "$API/accounts/$CF_ACCOUNT/r2/buckets/$BUCKET/objects?per_page=1000" \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" > "$WORK/list.json"
  python3 - "$WORK/list.json" "$WORK/keep.txt" <<'PY' > "$WORK/stale.txt"
import json, sys
doc = json.load(open(sys.argv[1]))
keep = set(open(sys.argv[2]).read().split())
for obj in doc.get("result", []):
    key = obj.get("key")
    if key and key not in keep:
        print(key)
PY
  stale_count=$(grep -c . "$WORK/stale.txt" || true)
  if [ "$stale_count" -eq 0 ]; then
    good "no stale objects"
  else
    while IFS= read -r k; do
      [ -n "$k" ] || continue
      api DELETE "/accounts/$CF_ACCOUNT/r2/buckets/$BUCKET/objects/$(urlenc "$k")" -X DELETE
      ok_or_die "delete $k"
      printf '    deleted %s\n' "$k"
    done < "$WORK/stale.txt"
    good "pruned $stale_count object(s)"
  fi
fi

# ---------------------------------------------------------------- workers ---

upload_worker() {
  local name="$1" script="$2" module="$3" bindings="$4"
  cat > "$WORK/meta.json" <<EOF
{"main_module":"$module","compatibility_date":"$COMPAT_DATE","compatibility_flags":[],"bindings":$bindings}
EOF
  if [ "$DRY_RUN" -eq 1 ]; then
    warn "would upload worker $name ($script)"
    return
  fi
  curl -sS --fail-with-body -X PUT "$API/accounts/$CF_ACCOUNT/workers/scripts/$name" \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
    --form "metadata=<$WORK/meta.json;type=application/json" \
    --form "$module=@$script;filename=$module;type=application/javascript+module" > "$WORK/resp.json"
  ok_or_die "upload worker $name"
  good "worker $name uploaded"
}

enable_subdomain() {
  local name="$1"
  if [ "$DRY_RUN" -eq 1 ]; then
    warn "would enable the workers.dev subdomain for $name"
    return
  fi
  curl -sS --fail-with-body -X POST "$API/accounts/$CF_ACCOUNT/workers/scripts/$name/subdomain" \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
    -H "Content-Type: application/json" \
    --data '{"enabled":true}' > "$WORK/resp.json"
  ok_or_die "enable subdomain for $name"
  good "$name is live on $name.$SUBDOMAIN.workers.dev"
}

step "App worker"
[ -f tools/worker-app.js ] || die "tools/worker-app.js is missing"
upload_worker "$APP_WORKER" tools/worker-app.js app.js \
  "[{\"name\":\"SITE\",\"type\":\"r2_bucket\",\"bucket_name\":\"$BUCKET\"}]"
enable_subdomain "$APP_WORKER"

step "Relay worker"
[ -f tools/worker-relay.js ] || die "tools/worker-relay.js is missing"
upload_worker "$RELAY_WORKER" tools/worker-relay.js worker-relay.js "[]"
enable_subdomain "$RELAY_WORKER"

# ----------------------------------------------------------------- verify ---

step "Verify"
APP_URL="https://$APP_WORKER.$SUBDOMAIN.workers.dev/"
RELAY_URL="https://$RELAY_WORKER.$SUBDOMAIN.workers.dev/?url=https%3A%2F%2Fwords.hk%2Fstatic%2Fjyutping%2Fho4.mp3"

if [ "$DRY_RUN" -eq 1 ]; then
  warn "would fetch $APP_URL"
  warn "would fetch $RELAY_URL"
else
  app_code=$(curl -sS -o /dev/null -w '%{http_code}' --max-time 15 "$APP_URL" || echo 000)
  [ "$app_code" = "200" ] || die "app worker returned $app_code"
  good "app      $app_code  $APP_URL"

  relay_code=$(curl -sS -o /dev/null -w '%{http_code}' --max-time 20 "$RELAY_URL" || echo 000)
  [ "$relay_code" = "200" ] || die "relay worker returned $relay_code"
  good "relay    $relay_code  $RELAY_WORKER.$SUBDOMAIN.workers.dev"

  blocked=$(curl -sS -o /dev/null -w '%{http_code}' --max-time 15 \
    "https://$RELAY_WORKER.$SUBDOMAIN.workers.dev/?url=https%3A%2F%2Fexample.com%2Fx.mp3" || echo 000)
  [ "$blocked" = "403" ] || warn "relay returned $blocked for a non-words.hk host (expected 403)"
  [ "$blocked" = "403" ] && good "relay    $blocked  non-words.hk host is refused"
fi

printf '\n%s%sDeployed.%s\n' "$BOLD" "$GREEN" "$OFF"
printf '    app    %s\n' "$APP_URL"
printf '    relay  https://%s.%s.workers.dev/?url=<encoded>\n' "$RELAY_WORKER" "$SUBDOMAIN"
