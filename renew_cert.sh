#!/usr/bin/env bash

set -euo pipefail

REMOTE_HOST="185.157.212.124"
REMOTE_USER="root"
DOMAIN="braintvsminigames.xyz"

echo "==> Stopping nginx to free port 80..."
ssh "${REMOTE_USER}@${REMOTE_HOST}" 'systemctl stop nginx || true'

echo "==> Renewing SSL certificate for ${DOMAIN}..."
ssh "${REMOTE_USER}@${REMOTE_HOST}" "certbot certonly --standalone -d ${DOMAIN} --force-renewal --non-interactive --agree-tos --email admin@${DOMAIN} --no-random-sleep-on-renew"

echo "==> Starting nginx..."
ssh "${REMOTE_USER}@${REMOTE_HOST}" 'systemctl start nginx'

echo "==> SSL certificate renewed successfully!"
