# transonic server

Optional standalone broker for transonic Connect features.

It does not replace an OpenSubsonic server and never reads media files or upstream
databases. Clients authenticate with their existing OpenSubsonic credentials or
API key; this server verifies them against the upstream server and stores only a
short-lived transonic session token hash.

## Development

```powershell
go test ./...
go run ./cmd/transonic-server help
go run ./cmd/transonic-server config print-default
go run ./cmd/transonic-server config generate-secret
go run ./cmd/transonic-server serve --config ./config.json
```

Set `sessionSigningSecret` in `config.json` to the value printed by
`config generate-secret`. If it is empty or still starts with `change-me-`, the
server will use a temporary signing secret and existing sessions will stop
working after restart.

## API

- `GET /healthz`
- `GET /version`
- `GET /v1/capabilities`
- `POST /v1/auth/login`
- `POST /v1/auth/refresh`
- `DELETE /v1/auth/logout`
- `GET /v1/ws`

Default port: `127.0.0.1:4747`.

## Linux systemd install

Install the current binary as a systemd service:

```sh
sudo ./transonic-server-linux-amd64 install --open-firewall
```

The installer copies the binary to `/usr/bin/transonic-server`, creates
`/etc/transonic-server/config.json` when it does not exist, writes the systemd
unit to `/etc/systemd/system/transonic-server.service`, creates
`/var/lib/transonic-server`, fixes ownership for existing data files, enables
the service, and starts it. The generated config listens on `0.0.0.0:4747`, uses
`/var/lib/transonic-server`, and includes a generated `sessionSigningSecret`.
If the service exits immediately, the installer fails and prints `systemctl`
and `journalctl` output.

To install an existing config:

```sh
sudo ./transonic-server-linux-amd64 install --config ./transonic.config.json --open-firewall
```

If `/etc/transonic-server/config.json` already exists, it is preserved. Pass
`--replace-config` only when you intentionally want to replace it.

Useful follow-up commands:

```sh
transonic-server service status
sudo transonic-server service uninstall
```

## Raspberry Pi smoke

Build on Windows:

```powershell
$env:GOOS='linux'; $env:GOARCH='arm64'; go build -o dist/transonic-server-linux-arm64 ./cmd/transonic-server
$env:GOOS='linux'; $env:GOARCH='arm'; $env:GOARM='7'; go build -o dist/transonic-server-linux-armv7 ./cmd/transonic-server
$env:GOOS='linux'; $env:GOARCH='amd64'; go build -o dist/transonic-server-linux-amd64 ./cmd/transonic-server
```

Copy `dist/transonic-server-linux-arm64` and `config.pi.example.json` to the Pi.

Run on Pi:

```sh
chmod +x ./transonic-server-linux-arm64
./transonic-server-linux-arm64 config generate-secret
./transonic-server-linux-arm64 serve --config ./config.pi.example.json
```

Client Connect URL: `http://<pi-ip>:4747`.
