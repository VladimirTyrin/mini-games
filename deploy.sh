#!/usr/bin/env bash

set -euo pipefail

REMOTE_HOST="185.157.212.124"
REMOTE_USER="root"
SERVICE_NAME="mini-games-server"
DEPLOY_DIR="/opt/mini-games-server"
BINARY_NAME="mini_games_server"
TARGET="x86_64-unknown-linux-musl"
WEB_CLIENT_DIR="web-client"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_FILE="${SCRIPT_DIR}/deploy/mini-games-server.service"
BUILD_BINARY="${SCRIPT_DIR}/target/${TARGET}/release/${BINARY_NAME}"
WEB_CLIENT_DIST="${SCRIPT_DIR}/${WEB_CLIENT_DIR}/dist"

echo "==> Building web client..."
cd "${SCRIPT_DIR}/${WEB_CLIENT_DIR}"
if [[ ! -d "node_modules" ]]; then
    echo "==> Installing npm dependencies..."
    npm install
fi
npm run build
cd "${SCRIPT_DIR}"

if [[ ! -d "${WEB_CLIENT_DIST}" ]]; then
    echo "Error: Web client dist not found at ${WEB_CLIENT_DIST}" >&2
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
EOF

echo "==> Deployment completed successfully!"
echo "==> gRPC server:  ${REMOTE_HOST}:5001"
echo "==> Web client:   http://${REMOTE_HOST}:5000/ui/"
echo ""
echo "Useful commands:"
echo "  View logs:    ssh ${REMOTE_USER}@${REMOTE_HOST} 'journalctl -u ${SERVICE_NAME} -f'"
echo "  Stop service: ssh ${REMOTE_USER}@${REMOTE_HOST} 'systemctl stop ${SERVICE_NAME}'"
echo "  Restart:      ssh ${REMOTE_USER}@${REMOTE_HOST} 'systemctl restart ${SERVICE_NAME}'"
