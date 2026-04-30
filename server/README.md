# transonic server

Optional standalone broker for transonic Connect features.

It does not replace an OpenSubsonic server and never reads media files or upstream
databases. Clients authenticate with their existing OpenSubsonic credentials or
API key; this server verifies them against the upstream server and stores only a
short-lived transonic session token hash.

## Development

```powershell
go test ./...
go run ./cmd/transonic-server config print-default
go run ./cmd/transonic-server serve --config ./config.json
```

## API

- `GET /healthz`
- `GET /version`
- `GET /v1/capabilities`
- `POST /v1/auth/login`
- `POST /v1/auth/refresh`
- `DELETE /v1/auth/logout`
- `GET /v1/ws`

Default port: `127.0.0.1:4747`.

## Raspberry Pi smoke

Build on Windows:

```powershell
$env:GOOS='linux'; $env:GOARCH='arm64'; go build -o dist/transonic-server-linux-arm64 ./cmd/transonic-server
$env:GOOS='linux'; $env:GOARCH='arm'; $env:GOARM='7'; go build -o dist/transonic-server-linux-armv7 ./cmd/transonic-server
```

Copy `dist/transonic-server-linux-arm64` and `config.pi.example.json` to the Pi.

Run on Pi:

```sh
chmod +x ./transonic-server-linux-arm64
./transonic-server-linux-arm64 serve --config ./config.pi.example.json
```

Client Connect URL: `http://<pi-ip>:4747`.
