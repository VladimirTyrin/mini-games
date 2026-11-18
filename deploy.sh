#!/usr/bin/env bash
# Deploys the server + web client to a Linux host over SSH.
#
#   ./deploy.sh                                   # production: server, web, APK and OTA bundle
#   ./deploy.sh --web-only                        # the site and the app's OTA bundle, ~30 s
#   ./deploy.sh --no-apk                          # server and web only
#   ./deploy.sh --host root@1.2.3.4 --mode http   # any host by IP, no TLS
#
# Machine-specific toolchain paths (JDK, Android SDK, zig) go to deploy.local.env next to this
# script (not in git; see deploy.local.env.example).
#
# Options:
#   --host H        ssh destination (default root@146.103.106.44)
#   --mode M        http | tls (default tls)
#   --domain D      domain for tls / server_name (default braintvsminigames.xyz)
#   --email E       ACME account email for tls (default admin@<domain>)
#   --no-apk        do not build or publish the Android app
#   --web-only      only the web client: the site and the over-the-air bundle for the app. No
#                   server build or restart (running matches go on), no APK. Refused when
#                   anything the server runs (crates, proto, Cargo files, deploy/) changed since
#                   the build the host serves, or when the app's native shell changed: those
#                   need the whole deploy
#   --apk FILE      publish this prebuilt APK instead of building one, with what
#                   build/android.mjs put next to it for the app's update check: android.json,
#                   and web.json + web-<build>.zip (over-the-air web bundle; 3 newest kept)
#   --allow-version-mismatch
#                   publish an APK built from another commit than the one being deployed
#   --skip-build    reuse existing build outputs
#   --dry-run       build and render configs locally, do not touch the host
#   --drain-seconds N
#                   before the restart, wait up to N seconds (default 180) for running matches
#                   to end; new ones are refused meanwhile. Matches live only in memory.
#   --no-drain      restart at once (same as --drain-seconds 0)
#
# Everything deployed carries one build number, the commit count of HEAD: the server reports it
# at /version, the web bundle in /version.json, the APK files in android.json and web.json. The
# deploy refuses mismatched inputs up front and checks what the host serves at the end.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="x86_64-unknown-linux-musl"
BINARY_NAME="mini_games_server"
SERVICE_NAME="mini-games-server"
REMOTE_DIR="/opt/mini-games-server"

HOST="root@146.103.106.44"
SSH_KEY="${HOME}/.ssh/id_ed25519_github"
GOOGLE_CLIENT_ID="996811598578-2dtc8gc90413c0qbtk87fhm84arqbrd3.apps.googleusercontent.com"
MODE="tls"
DOMAIN="braintvsminigames.xyz"
EMAIL=""
SKIP_BUILD=false
DRY_RUN=false
APK=""
BUILD_APK=true
ALLOW_VERSION_MISMATCH=false
DRAIN_SECONDS=180
WEB_ONLY=false

die() { echo "error: $*" >&2; exit 1; }
# The "build" number of a JSON document on stdin (android.json, web.json, version.json).
json_build() { grep -o '"build": *[0-9]*' | head -n 1 | grep -o '[0-9]*$' || true; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --host) HOST="$2"; shift 2 ;;
        --mode) MODE="$2"; shift 2 ;;
        --domain) DOMAIN="$2"; shift 2 ;;
        --email) EMAIL="$2"; shift 2 ;;
        --apk) APK="$2"; BUILD_APK=false; shift 2 ;;
        --allow-version-mismatch) ALLOW_VERSION_MISMATCH=true; shift ;;
        --skip-build) SKIP_BUILD=true; shift ;;
        --dry-run) DRY_RUN=true; shift ;;
        --no-apk) BUILD_APK=false; shift ;;
        --drain-seconds) DRAIN_SECONDS="$2"; shift 2 ;;
        --no-drain) DRAIN_SECONDS=0; shift ;;
        --web-only) WEB_ONLY=true; BUILD_APK=false; shift ;;
        -h|--help) sed -n '2,37p' "$0"; exit 0 ;;
        *) die "unknown option: $1" ;;
    esac
done

# shellcheck source=/dev/null
[[ -f "${SCRIPT_DIR}/deploy.local.env" ]] && source "${SCRIPT_DIR}/deploy.local.env"
[[ "$SKIP_BUILD" == true && "$BUILD_APK" == true ]] && BUILD_APK=false

SSH_OPTS=(-o IdentitiesOnly=yes -i "$SSH_KEY")
ssh() { command ssh "${SSH_OPTS[@]}" "$@"; }
scp() { command scp "${SSH_OPTS[@]}" "$@"; }
[[ "$MODE" == "http" || "$MODE" == "tls" ]] || die "--mode must be http or tls"
[[ "$DRAIN_SECONDS" =~ ^[0-9]+$ ]] || die "--drain-seconds takes a number of seconds"
[[ -z "$APK" || -f "$APK" ]] || die "APK not found: $APK"
if [[ "$MODE" == "tls" ]]; then
    [[ -n "$DOMAIN" ]] || die "--domain is required for --mode tls"
    EMAIL="${EMAIL:-admin@${DOMAIN}}"
fi
if [[ "$MODE" == "http" ]]; then SERVER_NAME="_"; else SERVER_NAME="$DOMAIN"; fi
if [[ "$MODE" == "tls" ]]; then PUBLIC_URL="https://${DOMAIN}"; else PUBLIC_URL="http://${HOST#*@}"; fi
if [[ "$WEB_ONLY" == true ]]; then
    [[ -z "$APK" && "$SKIP_BUILD" == false && "$ALLOW_VERSION_MISMATCH" == false ]]         || die "--web-only builds and publishes the web client alone: no --apk, --skip-build or --allow-version-mismatch"
fi

BUILD="$(git -C "$SCRIPT_DIR" rev-list --count HEAD)" || die "cannot count commits for the build number"
COMMIT="$(git -C "$SCRIPT_DIR" rev-parse --short HEAD)"
BUILD_BINARY="${SCRIPT_DIR}/target/${TARGET}/release/${BINARY_NAME}"
WEB_DIST="${SCRIPT_DIR}/client/dist"
echo "==> Build ${BUILD} (commit ${COMMIT})"
[[ -z "$(git -C "$SCRIPT_DIR" status --porcelain)" ]]     || echo "warning: uncommitted changes are deployed as build ${BUILD} (${COMMIT})" >&2

# The web client plays by the same rules as the server (the WASM core, the protocol): it may go
# out alone only while nothing the server runs differs from the build the host serves.
if [[ "$WEB_ONLY" == true ]]; then
    SERVED_SERVER="$(curl -fsS "${PUBLIC_URL}/version")" || die "cannot read the server's build at ${PUBLIC_URL}/version"
    SERVER_BUILD="${SERVED_SERVER##*+}"
    [[ "$SERVER_BUILD" =~ ^[0-9]+$ ]] || die "the server reports no build number (${SERVED_SERVER}): run ./deploy.sh"
    SERVER_COMMIT=""
    while read -r commit; do
        count="$(git -C "$SCRIPT_DIR" rev-list --count "$commit")"
        if (( count == SERVER_BUILD )); then SERVER_COMMIT="$commit"; break; fi
        (( count < SERVER_BUILD )) && break
    done < <(git -C "$SCRIPT_DIR" rev-list --first-parent HEAD)
    [[ -n "$SERVER_COMMIT" ]]         || die "the server runs build ${SERVER_BUILD}, which is not an earlier build of this branch: run ./deploy.sh"
    # Against the working tree, so uncommitted changes count too.
    SERVER_SIDE_CHANGES="$(git -C "$SCRIPT_DIR" diff --name-only "$SERVER_COMMIT" -- crates proto Cargo.toml Cargo.lock deploy deploy.sh)"
    [[ -z "$SERVER_SIDE_CHANGES" ]]         || die "changed since the server's build ${SERVER_BUILD}, so the server must go out too (run ./deploy.sh): $(echo "$SERVER_SIDE_CHANGES" | head -n 5 | tr '
' ' ')"
    echo "==> Web only: the server stays build ${SERVER_BUILD} ($(git -C "$SCRIPT_DIR" rev-parse --short "$SERVER_COMMIT")), nothing it runs changed since"
fi

if [[ "$BUILD_APK" == true ]]; then
    # First: it builds the web bundle for the app into client/dist, which the web build replaces.
    echo "==> Building Android app"
    (
        cd "${SCRIPT_DIR}/client"
        [[ -d node_modules ]] || npm ci
        npm run android:apk
    )
    APK="${SCRIPT_DIR}/client/android/dist/mini-games.apk"
fi
APK_DIR="$(dirname "${APK:-.}")"

# android.json and web.json must come from this commit too, before anything is uploaded.
if [[ -n "$APK" ]]; then
    for info in "${APK_DIR}/android.json" "${APK_DIR}/web.json"; do
        [[ -f "$info" ]] || continue
        info_build="$(json_build < "$info")"
        if [[ "$info_build" != "$BUILD" ]]; then
            [[ "$ALLOW_VERSION_MISMATCH" == true ]] \
                || die "$info is build ${info_build:-?}, this commit is build ${BUILD}: rebuild the APK from this commit (npm run android:apk) or pass --allow-version-mismatch"
            echo "warning: $info is build ${info_build:-?}, deploying build ${BUILD} (--allow-version-mismatch)" >&2
        fi
    done
fi

if [[ "$WEB_ONLY" == true ]]; then
    # First, like the APK: its web build goes to client/dist, which the site's build replaces.
    echo "==> Building the app's over-the-air web bundle"
    (
        cd "${SCRIPT_DIR}/client"
        [[ -d node_modules ]] || npm ci
        MG_BUILD_NUMBER="$BUILD" MG_COMMIT="$COMMIT" npm run android:bundle
    )
    APK_DIR="${SCRIPT_DIR}/client/android/dist"
    # Apps install a bundle only into the shell it names; a changed shell needs a new APK.
    BUNDLE_SHELL="$(grep -o '"nativeFingerprint": *"[^"]*"' "${APK_DIR}/web.json" | grep -o '"[^"]*"$' || true)"
    PUBLISHED_SHELL="$(curl -fsS "${PUBLIC_URL}/downloads/web.json" | grep -o '"nativeFingerprint": *"[^"]*"' | grep -o '"[^"]*"$' || true)"
    [[ -n "$BUNDLE_SHELL" && "$BUNDLE_SHELL" == "$PUBLISHED_SHELL" ]]         || die "the Android app's native shell changed (${PUBLISHED_SHELL:-none published} -> ${BUNDLE_SHELL:-?}): run ./deploy.sh to publish a new APK"
fi

if [[ "$SKIP_BUILD" == false ]]; then
    echo "==> Building web client"
    (
        cd "${SCRIPT_DIR}/client"
        [[ -d node_modules ]] || npm ci
        # The WASM core and protocol code are generated, not in git: build them from this commit
        # (the over-the-air bundle just did).
        [[ "$WEB_ONLY" == true ]] || npm run generate
        # No VITE_SERVER_URL: the page served by this host talks to the same host (/ws).
        MG_BUILD_NUMBER="$BUILD" MG_COMMIT="$COMMIT" npm run build
    )
    if [[ "$WEB_ONLY" == false ]]; then
        echo "==> Building server (${TARGET})"
        rustup target list --installed | grep -qx "${TARGET}" || rustup target add "${TARGET}"
        # zig provides the musl C toolchain that ring (TLS for Google key fetches) needs.
        MG_BUILD_NUMBER="$BUILD" cargo zigbuild --release --target "${TARGET}" -p mg-server
    fi
fi

[[ "$WEB_ONLY" == true || -f "$BUILD_BINARY" ]] || die "binary not found: $BUILD_BINARY"
[[ -f "$WEB_DIST/index.html" ]] || die "web build not found: $WEB_DIST"
WEB_BUILD="$(json_build < "$WEB_DIST/version.json" 2>/dev/null || true)"
[[ "$WEB_BUILD" == "$BUILD" ]] \
    || die "client/dist is build ${WEB_BUILD:-?}, this commit is build ${BUILD}: deploy without --skip-build"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
sed "s/__SERVER_NAME__/${SERVER_NAME}/g" "${SCRIPT_DIR}/deploy/nginx-http.conf.template" > "${STAGE}/site-http.conf"
if [[ "$MODE" == "tls" ]]; then
    sed -e "s/__SERVER_NAME__/${SERVER_NAME}/g" -e "s/__DOMAIN__/${DOMAIN}/g" \
        "${SCRIPT_DIR}/deploy/nginx-tls.conf.template" > "${STAGE}/site-tls.conf"
fi

if [[ "$DRY_RUN" == true ]]; then
    echo "==> Dry run: rendered configs in ${STAGE}"
    cat "${STAGE}"/site-*.conf
    trap - EXIT
    exit 0
fi

# The Firebase service account (push to the Android app) goes to the host when .keys has it;
# without it the host keeps the one it has. A new key can be made in the Firebase console at any
# time (Project settings, Service accounts). Written after the dry-run exit, which keeps $STAGE.
shopt -s nullglob
FCM_ACCOUNTS=("${SCRIPT_DIR}"/.keys/*-firebase-adminsdk-*.json)
shopt -u nullglob
(( ${#FCM_ACCOUNTS[@]} <= 1 )) || die "more than one Firebase service account key in .keys: keep one"
if (( ${#FCM_ACCOUNTS[@]} == 1 )); then
    FCM_ACCOUNT_BASE64="$(base64 < "${FCM_ACCOUNTS[0]}" | tr -d '\n')"
    [[ -n "$FCM_ACCOUNT_BASE64" ]] || die "cannot read ${FCM_ACCOUNTS[0]}"
    (umask 077; echo "MG_FCM_SERVICE_ACCOUNT=${FCM_ACCOUNT_BASE64}" > "${STAGE}/push.env")
    unset FCM_ACCOUNT_BASE64
fi

# The over-the-air web bundle: the zip first, web.json (which names it) last. Older zips stay for
# apps that read the previous web.json moments ago; the 3 newest are kept.
publish_web_bundle() {
    local WEB_INFO="${APK_DIR}/web.json" WEB_ZIP
    [[ -f "$WEB_INFO" ]] || return 0
    WEB_ZIP="$(sed -n 's/^ *"zip": *"\(web-[0-9]*\.zip\)".*/\1/p' "$WEB_INFO")"
    [[ -n "$WEB_ZIP" && -f "${APK_DIR}/${WEB_ZIP}" ]] || die "web.json names no zip next to it: $WEB_INFO"
    scp -q "${APK_DIR}/${WEB_ZIP}" "${HOST}:${REMOTE_DIR}/downloads/${WEB_ZIP}.new"
    ssh "$HOST" "chmod 644 ${REMOTE_DIR}/downloads/${WEB_ZIP}.new && mv ${REMOTE_DIR}/downloads/${WEB_ZIP}.new ${REMOTE_DIR}/downloads/${WEB_ZIP}"
    scp -q "$WEB_INFO" "${HOST}:${REMOTE_DIR}/downloads/web.json.new"
    ssh "$HOST" "chmod 644 ${REMOTE_DIR}/downloads/web.json.new && mv ${REMOTE_DIR}/downloads/web.json.new ${REMOTE_DIR}/downloads/web.json"
    ssh "$HOST" "cd ${REMOTE_DIR}/downloads && ls -1 web-*.zip | sort -t- -k2 -n | head -n -3 | xargs -r rm -f --"
}

MISMATCHES=()
check_build() { # name, served build, expected build
    if [[ "$2" == "$3" ]]; then echo "    ok  $1: build $2"; else MISMATCHES+=("$1 is build ${2:-?}, expected $3"); fi
}
fail_on_mismatches() {
    if (( ${#MISMATCHES[@]} )); then
        printf 'error: %s
' "${MISMATCHES[@]}" >&2
        die "${HOST} does not serve one build: see above"
    fi
}

echo "==> Uploading to ${HOST}"
ssh "$HOST" "mkdir -p ${REMOTE_DIR}/web.new ${REMOTE_DIR}/downloads /var/www/certbot && rm -rf ${REMOTE_DIR}/web.new/*"
tar -C "$WEB_DIST" -czf - . | ssh "$HOST" "tar --no-same-owner -xzf - -C ${REMOTE_DIR}/web.new"

if [[ "$WEB_ONLY" == true ]]; then
    publish_web_bundle
    # nginx serves web/ from disk: swapping the directory is the whole release. The server and
    # its matches are not touched.
    echo "==> Switching the site to build ${BUILD}"
    ssh "$HOST" "cd ${REMOTE_DIR} && rm -rf web.old && if [ -d web ]; then mv web web.old; fi && mv web.new web && chmod -R a+rX web && rm -rf web.old"
    echo "==> Checking what ${HOST} serves"
    check_build "web ${PUBLIC_URL}/version.json" "$(curl -fsS "${PUBLIC_URL}/version.json" | json_build)" "$BUILD"
    check_build "${PUBLIC_URL}/downloads/web.json" "$(curl -fsS "${PUBLIC_URL}/downloads/web.json" | json_build)" "$BUILD"
    SERVED_SERVER="$(curl -fsS "${PUBLIC_URL}/version" || true)"
    check_build "server ${PUBLIC_URL}/version, unchanged" "${SERVED_SERVER##*+}" "$SERVER_BUILD"
    fail_on_mismatches
    echo "==> Deployed the web client, build ${BUILD} (${COMMIT}), to ${HOST}; the server stays build ${SERVER_BUILD}"
    exit 0
fi
scp -q "$BUILD_BINARY" "${HOST}:${REMOTE_DIR}/${BINARY_NAME}.new"
if [[ -n "$APK" ]]; then
    scp -q "$APK" "${HOST}:${REMOTE_DIR}/downloads/mini-games.apk.new"
    ssh "$HOST" "chmod 644 ${REMOTE_DIR}/downloads/mini-games.apk.new && mv ${REMOTE_DIR}/downloads/mini-games.apk.new ${REMOTE_DIR}/downloads/mini-games.apk"
    # Published after the APK, so the app never offers a build that is not there yet.
    APK_INFO="${APK_DIR}/android.json"
    if [[ -f "$APK_INFO" ]]; then
        scp -q "$APK_INFO" "${HOST}:${REMOTE_DIR}/downloads/android.json.new"
        ssh "$HOST" "chmod 644 ${REMOTE_DIR}/downloads/android.json.new && mv ${REMOTE_DIR}/downloads/android.json.new ${REMOTE_DIR}/downloads/android.json"
    fi
    publish_web_bundle
fi
scp -q "${SCRIPT_DIR}/deploy/${SERVICE_NAME}.service" "${HOST}:/etc/systemd/system/${SERVICE_NAME}.service"
scp -q "${SCRIPT_DIR}/deploy/nginx-locations.conf" "${SCRIPT_DIR}/deploy/nginx-ws.conf" "${STAGE}"/site-*.conf "${HOST}:/tmp/"
if [[ -f "${STAGE}/push.env" ]]; then
    ssh "$HOST" "umask 077 && cat > /tmp/mini-games-push.env" < "${STAGE}/push.env"
fi

echo "==> Configuring host (${MODE})"
ssh "$HOST" MODE="$MODE" DOMAIN="$DOMAIN" EMAIL="$EMAIL" REMOTE_DIR="$REMOTE_DIR" \
    SERVICE_NAME="$SERVICE_NAME" BINARY_NAME="$BINARY_NAME" GOOGLE_CLIENT_ID="$GOOGLE_CLIENT_ID" \
    DRAIN_SECONDS="$DRAIN_SECONDS" bash -s <<'REMOTE'
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

need_pkgs=()
command -v nginx >/dev/null || need_pkgs+=(nginx)
if [[ "$MODE" == "tls" ]]; then command -v certbot >/dev/null || need_pkgs+=(certbot); fi
if (( ${#need_pkgs[@]} )); then
    apt-get update -qq
    apt-get install -y -qq "${need_pkgs[@]}"
fi

# Server settings live on the host only. The session secret signs session tokens: it is created
# once and kept, since replacing it signs every Google user out.
ENV_FILE=/etc/${SERVICE_NAME}.env
if [[ ! -f "$ENV_FILE" ]]; then
    umask 077
    {
        echo "MG_SESSION_SECRET=$(od -An -tx1 -N32 /dev/urandom | tr -d ' \n')"
        echo "MG_ALLOW_GUESTS=true"
        echo "MG_BOT_KEYS="
    } > "$ENV_FILE"
fi
# The client ID is public and follows the repository.
sed -i '/^MG_GOOGLE_CLIENT_ID=/d' "$ENV_FILE"
echo "MG_GOOGLE_CLIENT_ID=${GOOGLE_CLIENT_ID}" >> "$ENV_FILE"
# The Web Push key is made once and kept like the session secret: browsers subscribed with its
# public half, and a new key makes them subscribe again. 32 random bytes are a valid P-256 key
# unless above the group order (chance about 2^-32; the server would refuse to start).
if ! grep -q '^MG_VAPID_PRIVATE_KEY=' "$ENV_FILE"; then
    echo "MG_VAPID_PRIVATE_KEY=$(od -An -tx1 -N32 /dev/urandom | tr -d ' \n')" >> "$ENV_FILE"
fi
sed -i '/^MG_VAPID_SUBJECT=/d' "$ENV_FILE"
# Without a site (plain http by IP) the server leaves Web Push off: browsers offer push to
# secure origins only.
if [[ "$MODE" == "tls" ]]; then
    echo "MG_VAPID_SUBJECT=https://${DOMAIN}" >> "$ENV_FILE"
fi
if [[ -f /tmp/mini-games-push.env ]]; then
    sed -i '/^MG_FCM_SERVICE_ACCOUNT=/d' "$ENV_FILE"
    cat /tmp/mini-games-push.env >> "$ENV_FILE"
    rm -f /tmp/mini-games-push.env
fi
chmod 600 "$ENV_FILE"

# Matches live only in the server's memory: let the running ones end before the restart. The
# server refuses new ones meanwhile and tells the players. An older server without the drain
# endpoint (or none running) answers nothing, and the restart goes ahead.
if (( DRAIN_SECONDS > 0 )); then
    waited=0
    while running="$(curl -fs -X POST http://127.0.0.1:5000/admin/drain 2>/dev/null)" \
        && [[ "$running" =~ ^[0-9]+$ ]] && (( running > 0 )); do
        if (( waited >= DRAIN_SECONDS )); then
            echo "==> ${running} match(es) still running after ${DRAIN_SECONDS} s: restarting anyway"
            break
        fi
        if (( waited % 30 == 0 )); then
            echo "==> Waiting for ${running} running match(es) to end before the restart"
        fi
        sleep 5
        waited=$((waited + 5))
    done
fi

# Swap in the new build.
chmod 755 "${REMOTE_DIR}/${BINARY_NAME}.new"
mv -f "${REMOTE_DIR}/${BINARY_NAME}.new" "${REMOTE_DIR}/${BINARY_NAME}"
rm -rf "${REMOTE_DIR}/web.old"
[[ -d "${REMOTE_DIR}/web" ]] && mv "${REMOTE_DIR}/web" "${REMOTE_DIR}/web.old"
mv "${REMOTE_DIR}/web.new" "${REMOTE_DIR}/web"
chmod -R a+rX "${REMOTE_DIR}"

install -m 644 /tmp/nginx-locations.conf /etc/nginx/snippets/mini-games-locations.conf
install -m 644 /tmp/nginx-ws.conf /etc/nginx/snippets/mini-games-ws.conf
rm -f /etc/nginx/sites-enabled/default

apply_site() {
    install -m 644 "/tmp/$1" /etc/nginx/sites-available/mini-games.conf
    ln -sf /etc/nginx/sites-available/mini-games.conf /etc/nginx/sites-enabled/mini-games.conf
    nginx -t -q
    systemctl enable -q nginx
    systemctl reload nginx 2>/dev/null || systemctl restart nginx
}

apply_site site-http.conf

if [[ "$MODE" == "tls" ]]; then
    if [[ ! -f "/etc/letsencrypt/live/${DOMAIN}/fullchain.pem" ]]; then
        echo "==> Requesting certificate for ${DOMAIN}"
        certbot certonly --webroot -w /var/www/certbot -d "${DOMAIN}" \
            --non-interactive --agree-tos -m "${EMAIL}" --no-eff-email
    fi
    mkdir -p /etc/letsencrypt/renewal-hooks/deploy
    printf '#!/bin/sh\nsystemctl reload nginx\n' > /etc/letsencrypt/renewal-hooks/deploy/reload-nginx.sh
    chmod 755 /etc/letsencrypt/renewal-hooks/deploy/reload-nginx.sh
    apply_site site-tls.conf
fi
rm -f /tmp/nginx-locations.conf /tmp/nginx-ws.conf /tmp/site-*.conf

systemctl daemon-reload
systemctl enable -q "${SERVICE_NAME}"
systemctl restart "${SERVICE_NAME}"

for _ in $(seq 1 20); do
    if curl -fs -o /dev/null http://127.0.0.1:5000/healthz 2>/dev/null; then break; fi
    sleep 0.5
done
if ! systemctl is-active --quiet "${SERVICE_NAME}"; then
    journalctl -u "${SERVICE_NAME}" -n 50 --no-pager
    exit 1
fi
# Each check on its own line: set -e ignores a failure inside an && chain. A redirect does not
# pass (plain http answers 301 in tls mode): the site itself must answer 200.
if [[ "$MODE" == "tls" ]]; then
    RESOLVE=(--resolve "${DOMAIN}:443:127.0.0.1")
    SITE="https://${DOMAIN}"
else
    RESOLVE=()
    SITE="http://127.0.0.1"
fi
for path in /healthz /; do
    status="$(curl -sS -o /dev/null -w '%{http_code}' "${RESOLVE[@]}" "${SITE}${path}" || true)"
    if [[ "$status" != 200 ]]; then
        echo "nginx answers ${SITE}${path} with ${status:-nothing}" >&2
        exit 1
    fi
done
echo "==> nginx OK"
rm -rf "${REMOTE_DIR}/web.old"
REMOTE

echo "==> Checking that ${HOST} serves build ${BUILD} everywhere"
SERVED_SERVER="$(curl -fsS "${PUBLIC_URL}/version" || true)"
check_build "server ${PUBLIC_URL}/version (${SERVED_SERVER:-unreachable})" "${SERVED_SERVER##*+}" "$BUILD"
check_build "web ${PUBLIC_URL}/version.json" "$(curl -fsS "${PUBLIC_URL}/version.json" | json_build)" "$BUILD"
if [[ -n "$APK" ]]; then
    # With --allow-version-mismatch the APK files are expected to carry their own build.
    for info in android.json web.json; do
        [[ -f "${APK_DIR}/${info}" ]] || continue
        expected="$BUILD"
        [[ "$ALLOW_VERSION_MISMATCH" == true ]] && expected="$(json_build < "${APK_DIR}/${info}")"
        check_build "${PUBLIC_URL}/downloads/${info}" "$(curl -fsS "${PUBLIC_URL}/downloads/${info}" | json_build)" "$expected"
    done
fi
fail_on_mismatches

echo "==> Deployed build ${BUILD} (${COMMIT}) to ${HOST} (${MODE})"
