#!/usr/bin/env bash

set -euo pipefail

SHOW_REMOTE_COMMANDS=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -rc|--remote-commands)
            SHOW_REMOTE_COMMANDS=true
            shift
            ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Usage: $0 [-rc|--remote-commands]" >&2
            exit 1
            ;;
    esac
done

REMOTE_HOST="185.157.212.124"
REMOTE_USER="root"
DOMAIN="braintvsminigames.xyz"
SERVICE_NAME="mini-games-server"
DEPLOY_DIR="/opt/mini-games-server"
BINARY_NAME="mini_games_server"
TARGET="x86_64-unknown-linux-musl"
WEB_CLIENT_DIR="web-client"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_FILE="${SCRIPT_DIR}/deploy/mini-games-server.service"
NGINX_CONF="${SCRIPT_DIR}/deploy/nginx.conf"
BUILD_BINARY="${SCRIPT_DIR}/target/${TARGET}/release/${BINARY_NAME}"
WEB_CLIENT_DIST="${SCRIPT_DIR}/${WEB_CLIENT_DIR}/dist"

if [[ "${SHOW_REMOTE_COMMANDS}" == "true" ]]; then
    echo "View logs:    ssh ${REMOTE_USER}@${REMOTE_HOST} 'journalctl -u ${SERVICE_NAME} -f'"
    echo "Nginx logs:   ssh ${REMOTE_USER}@${REMOTE_HOST} 'tail -f /var/log/nginx/error.log'"
    echo "Stop service: ssh ${REMOTE_USER}@${REMOTE_HOST} 'systemctl stop ${SERVICE_NAME}'"
    echo "Restart:      ssh ${REMOTE_USER}@${REMOTE_HOST} 'systemctl restart ${SERVICE_NAME}'"
    echo "Renew SSL:    ssh ${REMOTE_USER}@${REMOTE_HOST} 'certbot renew'"
    exit 0
fi

echo "==> Building web client..."
cd "${SCRIPT_DIR}/${WEB_CLIENT_DIR}"
if [[ ! -d "node_modules" ]]; then
    echo "==> Installing npm dependencies..."
    npm install
fi
VITE_SERVER_URL="wss://${DOMAIN}/ws" npm run build
cd "${SCRIPT_DIR}"

if [[ ! -d "${WEB_CLIENT_DIST}" ]]; then
    echo "Error: Web client dist not found at ${WEB_CLIENT_DIST}" >&2
    exit 1
fi

echo "==> Running tests (release mode)..."
cargo test --release
if [[ $? -ne 0 ]]; then
    echo "Error: Tests failed" >&2
    exit 1
fi

echo "==> Building server for Linux (${TARGET})..."
if ! rustup target list --installed | grep -q "${TARGET}"; then
    echo "==> Installing Rust target: ${TARGET}"
    rustup target add "${TARGET}"
fi

cargo build --release --target "${TARGET}" -p mini_games_server

if [[ ! -f "${BUILD_BINARY}" ]]; then
    echo "Error: Binary not found at ${BUILD_BINARY}" >&2
    exit 1
fi

echo "==> Binary size: $(du -h "${BUILD_BINARY}" | cut -f1)"

echo "==> Deploying to ${REMOTE_USER}@${REMOTE_HOST}..."

ssh "${REMOTE_USER}@${REMOTE_HOST}" bash <<EOF
set -euo pipefail

echo "==> Creating deployment directory..."
mkdir -p "${DEPLOY_DIR}"
mkdir -p "${DEPLOY_DIR}/web-client"

echo "==> Stopping service if running..."
if systemctl is-active --quiet "${SERVICE_NAME}"; then
    systemctl stop "${SERVICE_NAME}"
    echo "==> Service stopped"
fi
EOF

echo "==> Copying binary to remote host..."
scp "${BUILD_BINARY}" "${REMOTE_USER}@${REMOTE_HOST}:${DEPLOY_DIR}/${BINARY_NAME}"

echo "==> Copying web client to remote host..."
scp -r "${WEB_CLIENT_DIST}" "${REMOTE_USER}@${REMOTE_HOST}:${DEPLOY_DIR}/web-client/"

echo "==> Copying systemd service file..."
scp "${SERVICE_FILE}" "${REMOTE_USER}@${REMOTE_HOST}:/etc/systemd/system/${SERVICE_NAME}.service"

echo "==> Ensuring nginx is installed..."
ssh "${REMOTE_USER}@${REMOTE_HOST}" bash <<'NGINX_INSTALL'
set -euo pipefail
if ! command -v nginx &> /dev/null; then
    echo "==> Installing nginx..."
    apt-get update
    apt-get install -y nginx
fi
mkdir -p /etc/nginx/sites-available /etc/nginx/sites-enabled
NGINX_INSTALL

echo "==> Copying nginx config..."
scp "${NGINX_CONF}" "${REMOTE_USER}@${REMOTE_HOST}:/etc/nginx/sites-available/${DOMAIN}"

ssh "${REMOTE_USER}@${REMOTE_HOST}" bash <<EOF
set -euo pipefail

echo "==> Setting permissions..."
chmod 755 "${DEPLOY_DIR}/${BINARY_NAME}"

echo "==> Reloading systemd daemon..."
systemctl daemon-reload

echo "==> Enabling service..."
systemctl enable "${SERVICE_NAME}"

echo "==> Starting service..."
systemctl start "${SERVICE_NAME}"

echo "==> Service status:"
systemctl status "${SERVICE_NAME}" --no-pager || true

echo "==> Checking if service is running..."
sleep 2
if systemctl is-active --quiet "${SERVICE_NAME}"; then
    echo "==> Service is running successfully!"
else
    echo "Error: Service failed to start" >&2
    journalctl -u "${SERVICE_NAME}" -n 50 --no-pager
    exit 1
fi

echo "==> Setting up SSL..."
if [[ ! -f /etc/letsencrypt/live/${DOMAIN}/fullchain.pem ]]; then
    echo "==> SSL certificate not found. Obtaining with certbot..."
    if ! command -v certbot &> /dev/null; then
        echo "==> Installing certbot..."
        apt-get update
        apt-get install -y certbot
    fi

    echo "==> Stopping nginx for standalone certificate..."
    systemctl stop nginx || true

    certbot certonly --standalone -d ${DOMAIN} --non-interactive --agree-tos --email admin@${DOMAIN} || {
        echo "Error: certbot failed. You may need to run it manually."
        echo "Make sure DNS is pointing to this server, then run:"
        echo "  certbot certonly --standalone -d ${DOMAIN}"
        exit 1
    }
fi

echo "==> Enabling nginx site..."
ln -sf /etc/nginx/sites-available/${DOMAIN} /etc/nginx/sites-enabled/${DOMAIN}
rm -f /etc/nginx/sites-enabled/default

echo "==> Testing nginx config..."
nginx -t

echo "==> Starting nginx..."
systemctl enable nginx
systemctl start nginx
EOF

echo "==> Deployment completed successfully!"
echo "==> Web client:   https://${DOMAIN}/"
echo "==> gRPC server:  https://${DOMAIN}:5443"
echo ""
echo "Useful commands:"
echo "  View logs:    ssh ${REMOTE_USER}@${REMOTE_HOST} 'journalctl -u ${SERVICE_NAME} -f'"
echo "  Nginx logs:   ssh ${REMOTE_USER}@${REMOTE_HOST} 'tail -f /var/log/nginx/error.log'"
echo "  Stop service: ssh ${REMOTE_USER}@${REMOTE_HOST} 'systemctl stop ${SERVICE_NAME}'"
echo "  Restart:      ssh ${REMOTE_USER}@${REMOTE_HOST} 'systemctl restart ${SERVICE_NAME}'"
echo "  Renew SSL:    ssh ${REMOTE_USER}@${REMOTE_HOST} 'certbot renew'"
