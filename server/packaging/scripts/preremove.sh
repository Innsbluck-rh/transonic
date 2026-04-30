#!/bin/sh
set -e

if command -v systemctl >/dev/null 2>&1; then
  systemctl stop transonic-server || true
  systemctl disable transonic-server || true
fi
