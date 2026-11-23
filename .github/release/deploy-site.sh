#!/usr/bin/env bash
# deploy-site.sh <dev|production> <dir>: deploys the site Worker from a
# verified artifact (<dir>, holding worker.ts and dist/ from Deploy's build
# job) with the wrangler CLI pinned in wrangler/package-lock.json, which the
# workflow installs (npm ci --ignore-scripts) before the token is in reach.
#
# The artifact was built from repository code without secrets, so everything
# in it is untrusted. The Worker's code and assets may change freely, but the
# config wrangler deploys is generated here and pins this mode's Worker name,
# account, compatibility flags, assets binding, and routes: the deploy token
# can write Workers on the account, and a binding grants the Worker access to
# its resource. Any config or unexpected file in the artifact is refused;
# changing Worker settings, routes, or bindings means changing this script.
. "$(dirname "$0")/lib.sh"

[ $# -eq 2 ] || die "Usage: deploy-site.sh <dev|production> <dir>"
mode=$1
case "$mode" in
  dev)
    worker=spaceman-dev-site host=dev.anoma.ly
    ;;
  production)
    worker=spaceman-production-site host=anoma.ly
    ;;
  *) die "Unknown site mode: $mode" ;;
esac
: "${CLOUDFLARE_API_TOKEN:?Set CLOUDFLARE_API_TOKEN}"
: "${CLOUDFLARE_ACCOUNT_ID:?Set CLOUDFLARE_ACCOUNT_ID}"
case "$CLOUDFLARE_ACCOUNT_ID" in
  *[!0-9a-f]*|"") die "CLOUDFLARE_ACCOUNT_ID must be a 32-character lowercase hex id." ;;
esac
[ ${#CLOUDFLARE_ACCOUNT_ID} -eq 32 ] || die "CLOUDFLARE_ACCOUNT_ID must be a 32-character lowercase hex id."

src=$(cd "$2" && pwd)
release=$(cd "$(dirname "$0")" && pwd)

# Only worker.ts and dist/ (plus SHA256SUMS from upload); nothing else is
# accepted or copied to where wrangler runs.
unexpected=$(cd "$src" && find . ! -type d ! -type f -o -type f ! -path ./SHA256SUMS \
  ! -path ./worker.ts \
  ! -path './dist/*')
[ -z "$unexpected" ] || die "Unexpected files in the site artifact: $unexpected"
[ -f "$src/worker.ts" ] || die "Missing worker.ts in the site artifact"
[ -f "$src/dist/index.html" ] || die "Missing dist/index.html in the site artifact"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp "$src/worker.ts" "$work/worker.ts"
cp -R "$src/dist" "$work/dist"

jq -n \
  --arg name "$worker" \
  --arg account_id "$CLOUDFLARE_ACCOUNT_ID" \
  --arg host "$host" \
  '{
    name: $name,
    account_id: $account_id,
    main: "./worker.ts",
    compatibility_date: "2026-08-05",
    compatibility_flags: ["global_fetch_strictly_public"],
    workers_dev: false,
    preview_urls: false,
    assets: {
      directory: "./dist",
      binding: "ASSETS",
      run_worker_first: true
    },
    routes: [
      { pattern: "\($host)/spaceman", zone_name: $host },
      { pattern: "\($host)/spaceman/*", zone_name: $host }
    ]
  }' > "$work/wrangler.jsonc"

wrangler="$release/wrangler/node_modules/.bin/wrangler"
[ -x "$wrangler" ] || die "Install the pinned wrangler CLI first: npm ci --ignore-scripts in .github/release/wrangler"
cd "$work" || exit 1
WRANGLER_SEND_METRICS=false "$wrangler" deploy --config "$work/wrangler.jsonc"
