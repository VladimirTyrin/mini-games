#!/usr/bin/env bash
# Deploys the server + web client to a Linux host over SSH.
#
#   ./deploy.sh                                   # production: TLS on braintvsminigames.xyz
#   ./deploy.sh --host root@1.2.3.4 --mode http   # any host by IP, no TLS
#
# Options:
#   --host H        ssh destination (default root@146.103.106.44)
#   --mode M        http | tls (default tls)
#   --domain D      domain for tls / server_name (default braintvsminigames.xyz)
#   --email E       ACME account email for tls (default admin@<domain>)
#   --skip-build    reuse existing build outputs
#   --dry-run       build and render configs locally, do not touch the host
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="x86_64-unknown-linux-musl"
BINARY_NAME="mini_games_server"
SERVICE_NAME="mini-games-server"
REMOTE_DIR="/opt/mini-games-server"

HOST="root@146.103.106.44"
SSH_KEY="${HOME}/.ssh/id_ed25519_github"
MODE="tls"
DOMAIN="braintvsminigames.xyz"
EMAIL=""
SKIP_BUILD=false
DRY_RUN=false

die() { echo "error: $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --host) HOST="$2"; shift 2 ;;
        --mode) MODE="$2"; shift 2 ;;
        --domain) DOMAIN="$2"; shift 2 ;;
        --email) EMAIL="$2"; shift 2 ;;
        --skip-build) SKIP_BUILD=true; shift ;;
        --dry-run) DRY_RUN=true; shift ;;
        -h|--help) sed -n '2,14p' "$0"; exit 0 ;;
        *) die "unknown option: $1" ;;
    esac
done

SSH_OPTS=(-o IdentitiesOnly=yes -i "$SSH_KEY")
ssh() { command ssh "${SSH_OPTS[@]}" "$@"; }
scp() { command scp "${SSH_OPTS[@]}" "$@"; }
[[ "$MODE" == "http" || "$MODE" == "tls" ]] || die "--mode must be http or tls"
if [[ "$MODE" == "tls" ]]; then
    [[ -n "$DOMAIN" ]] || die "--domain is required for --mode tls"
    EMAIL="${EMAIL:-admin@${DOMAIN}}"
fi
if [[ "$MODE" == "http" ]]; then SERVER_NAME="_"; else SERVER_NAME="$DOMAIN"; fi

BUILD_BINARY="${SCRIPT_DIR}/target/${TARGET}/release/${BINARY_NAME}"
WEB_DIST="${SCRIPT_DIR}/web-client/dist"

if [[ "$SKIP_BUILD" == false ]]; then
    echo "==> Building web client"
    (
        cd "${SCRIPT_DIR}/web-client"
        [[ -d node_modules ]] || npm ci
        npm run generate:proto
        # No VITE_SERVER_URL: the page served by this host talks to the same host (/ws).
        npm run build
    )
    echo "==> Building server (${TARGET})"
    rustup target list --installed | grep -qx "${TARGET}" || rustup target add "${TARGET}"
    cargo build --release --target "${TARGET}" -p mini_games_server
fi

[[ -f "$BUILD_BINARY" ]] || die "binary not found: $BUILD_BINARY"
[[ -f "$WEB_DIST/index.html" ]] || die "web build not found: $WEB_DIST"

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

echo "==> Uploading to ${HOST}"
ssh "$HOST" "mkdir -p ${REMOTE_DIR}/web.new /var/www/certbot && rm -rf ${REMOTE_DIR}/web.new/*"
tar -C "$WEB_DIST" -czf - . | ssh "$HOST" "tar -xzf - -C ${REMOTE_DIR}/web.new"
scp -q "$BUILD_BINARY" "${HOST}:${REMOTE_DIR}/${BINARY_NAME}.new"
scp -q "${SCRIPT_DIR}/deploy/${SERVICE_NAME}.service" "${HOST}:/etc/systemd/system/${SERVICE_NAME}.service"
scp -q "${SCRIPT_DIR}/deploy/nginx-locations.conf" "${STAGE}"/site-*.conf "${HOST}:/tmp/"

echo "==> Configuring host (${MODE})"
ssh "$HOST" MODE="$MODE" DOMAIN="$DOMAIN" EMAIL="$EMAIL" REMOTE_DIR="$REMOTE_DIR" \
    SERVICE_NAME="$SERVICE_NAME" BINARY_NAME="$BINARY_NAME" bash -s <<'REMOTE'
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

need_pkgs=()
command -v nginx >/dev/null || need_pkgs+=(nginx)
if [[ "$MODE" == "tls" ]]; then command -v certbot >/dev/null || need_pkgs+=(certbot); fi
if (( ${#need_pkgs[@]} )); then
    apt-get update -qq
    apt-get install -y -qq "${need_pkgs[@]}"
fi

# Swap in the new build.
chmod 755 "${REMOTE_DIR}/${BINARY_NAME}.new"
mv -f "${REMOTE_DIR}/${BINARY_NAME}.new" "${REMOTE_DIR}/${BINARY_NAME}"
rm -rf "${REMOTE_DIR}/web.old"
[[ -d "${REMOTE_DIR}/web" ]] && mv "${REMOTE_DIR}/web" "${REMOTE_DIR}/web.old"
mv "${REMOTE_DIR}/web.new" "${REMOTE_DIR}/web"
chmod -R a+rX "${REMOTE_DIR}"

install -m 644 /tmp/nginx-locations.conf /etc/nginx/snippets/mini-games-locations.conf
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
rm -f /tmp/nginx-locations.conf /tmp/site-*.conf

systemctl daemon-reload
systemctl enable -q "${SERVICE_NAME}"
systemctl restart "${SERVICE_NAME}"

for _ in $(seq 1 20); do
    if curl -fs -o /dev/null http://127.0.0.1:5000/ui/ 2>/dev/null; then break; fi
    sleep 0.5
done
if ! systemctl is-active --quiet "${SERVICE_NAME}"; then
    journalctl -u "${SERVICE_NAME}" -n 50 --no-pager
    exit 1
fi
curl -fsS -o /dev/null http://127.0.0.1/ui/ && echo "==> nginx -> server OK"
rm -rf "${REMOTE_DIR}/web.old"
REMOTE

echo "==> Deployed to ${HOST} (${MODE})"
