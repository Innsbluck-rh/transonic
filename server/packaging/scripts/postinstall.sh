#!/bin/sh
set -e

if ! getent group transonic-server >/dev/null 2>&1; then
  groupadd --system transonic-server >/dev/null 2>&1 || true
fi

if ! id transonic-server >/dev/null 2>&1; then
  useradd --system --no-create-home --gid transonic-server --home-dir /var/lib/transonic-server --shell /usr/sbin/nologin transonic-server >/dev/null 2>&1 || true
fi

mkdir -p /etc/transonic-server /var/lib/transonic-server
chown transonic-server:transonic-server /var/lib/transonic-server
chmod 0750 /etc/transonic-server /var/lib/transonic-server

if command -v systemctl >/dev/null 2>&1; then
  systemctl daemon-reload || true
  systemctl enable transonic-server || true
fi
