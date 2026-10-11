# transonic server

transonic server enables Transonic Connect: sharing a playback queue and controlling
playback across devices. Run one Connect server for the devices you want to connect.
You still need a Subsonic/OpenSubsonic music server and an account on it.

This guide covers installation on Linux with systemd, including Raspberry Pi.
You can build on the Linux machine itself or build on Windows and copy the binary.

- [Install](#install)
- [Connect the app](#connect-the-app)
- [Update or reinstall](#update-or-reinstall)
- [Configure and manage the service](#configure-and-manage-the-service)
- [Run without installing a service](#run-without-installing-a-service)
- [Troubleshooting](#troubleshooting)
- [Uninstall](#uninstall)

## Install

### 1. Prepare

On the **Linux server**, you need systemd and sudo/root access. The server must be
reachable from your client devices, and it must be able to reach the music server
at the URL used by those clients.

On the **build machine**, install [Go](https://go.dev/doc/install) 1.23 or later
and Git. If you do not already have the source:

```sh
git clone https://github.com/Innsbluck-rh/transonic.git
cd transonic
```

The build commands below start in the **repository root** (`transonic`), not in
`server`. Choose **one** of the following build methods.

### 2A. Build directly on the Linux server

Run in the repository root on the Linux server:

```sh
CGO_ENABLED=0 go -C server build -o dist/transonic-server ./cmd/transonic-server
cd server/dist
```

The binary is now `server/dist/transonic-server`. Continue to [step 3](#3-install-and-start).

### 2B. Build on Windows and copy to Linux

First, run this on the **Linux server** to identify its architecture:

```sh
uname -m
getconf LONG_BIT
```

Choose the target for the installed OS. A Pi running a 32-bit OS needs the ARMv7
build even if its processor supports 64-bit operation.

| Linux target | `GOARCH` | `GOARM` |
| --- | --- | --- |
| 64-bit Raspberry Pi / ARM64 (`aarch64`, 64-bit OS) | `arm64` | `7` (unused) |
| 32-bit ARMv7 (`armv7l`, or a Pi with a 32-bit ARMv7 OS) | `arm` | `7` |
| 64-bit Intel / AMD (`x86_64`) | `amd64` | `7` (unused) |

Open a **new PowerShell window** in the repository root on Windows. This example
builds for a 64-bit Raspberry Pi; change `GOARCH` using the table if needed:

```powershell
$env:GOOS = 'linux'
$env:GOARCH = 'arm64'
$env:GOARM = '7'
$env:CGO_ENABLED = '0'
go -C server build -o dist/transonic-server ./cmd/transonic-server
```

After the build succeeds, copy the binary and log in. Replace `YOUR_USER` and
`SERVER_IP` with your Linux SSH username and server address:

```powershell
scp ./server/dist/transonic-server YOUR_USER@SERVER_IP:~/transonic-server
ssh YOUR_USER@SERVER_IP
```

In that **Linux SSH session**:

```sh
cd ~
```

Continue below. Close the PowerShell window after you finish so its Linux build
environment does not affect subsequent Windows builds. Go is only needed on the
build machine.

### 3. Install and start

On the **Linux server**, in the directory containing the binary:

```sh
chmod +x ./transonic-server
sudo ./transonic-server install
```

This installs the binary, creates a service account and configuration, enables
startup at boot, and starts the server. No manual configuration or secret generation
is needed for a first installation on a trusted LAN.

| Item | Installed location / value |
| --- | --- |
| Binary | `/usr/bin/transonic-server` |
| Configuration | `/etc/transonic-server/config.json` |
| Data | `/var/lib/transonic-server` |
| systemd unit | `/etc/systemd/system/transonic-server.service` |
| Listen address for a new installation | `0.0.0.0:4747` (HTTP on all IPv4 interfaces) |

Existing configuration and data are kept when you run `install` again.

Allow inbound TCP port **4747** from your client devices if a firewall blocks it.
If you use **firewalld**, the installer can add the rule:

```sh
sudo ./transonic-server install --open-firewall
```

This option requires running firewalld. It selects the zone assigned to `tailscale0`
when available, otherwise the default zone. To choose a zone explicitly, add
`--firewall-zone YOUR_ZONE`. With another firewall, configure the rule there.

### 4. Check the installation

On the Linux server:

```sh
transonic-server service status
curl --fail http://127.0.0.1:4747/healthz
```

The service should show `active (running)`, and the HTTP response should be
`{"status":"ok"}`. From a client device, open `http://SERVER_IP:4747/healthz`
and check for the same response. Then [connect the app](#connect-the-app).

## Connect the app

On each device:

1. Sign in to your music server in Transonic. Use the same music server URL and
   account on the devices that should share playback.
2. Open **Settings → Connect** and enable **Use Transonic Connect**.
3. Open **Advanced**. For the HTTP server installed above, turn off
   **Use Subsonic Host URL**, enter `http://SERVER_IP` in **Connect Server URL**,
   and set **Port** to `4747`.
4. Enable **Allow LAN HTTP Connect Server** for this trusted LAN connection.
5. Set a recognizable **Device Name**, click **Apply**, and check that the status
   becomes **Connected**.

Use the Connect server's reachable IP address or hostname, not `0.0.0.0`.
`localhost` refers to the device running the app.

You can leave **Use Subsonic Host URL** enabled when the music server and Connect
server use the same host **and URL scheme**. The port is still set separately.
For example, an HTTPS music server URL does not match a plain HTTP Connect server.

Connect uses your existing music-server login; there is no separate Connect account
to create. API-key login requires the music server to support
[OpenSubsonic API Key Authentication](https://opensubsonic.netlify.app/docs/extensions/apikeyauth/).

## Update or reinstall

On the build machine, start in the repository root. To update your current branch
from its tracked remote branch:

```sh
git pull --ff-only
```

If this fails, resolve the checkout state before continuing. For a reinstall of
the same source revision, skip the pull.

Repeat [step 2A](#2a-build-directly-on-the-linux-server) or
[step 2B](#2b-build-on-windows-and-copy-to-linux) to rebuild and, if necessary,
copy the new binary. On the Linux server, run **the newly built/copied binary**:

```sh
chmod +x ./transonic-server
sudo ./transonic-server install
transonic-server service status
curl --fail http://127.0.0.1:4747/healthz
```

Run these commands from `server/dist` for a local build, or from your home directory
for the SSH copy above. If you changed the listen address, port, or TLS settings,
use your configured URL for the health check. The installer replaces the binary and restarts
the service. Playback control is briefly disconnected during the restart.

You do not need to uninstall first. The existing configuration, signing secret,
and data are preserved. Do not use `--replace-config` for a routine update.
Running `/usr/bin/transonic-server install` reinstalls that already-installed binary;
it does not pick up your new build.

## Configure and manage the service

Edit `/etc/transonic-server/config.json` with elevated privileges, then restart:

```sh
sudo systemctl restart transonic-server
transonic-server service status
```

Common settings:

| Setting | When to change it |
| --- | --- |
| `listenAddress` | Change the bind address or port, for example `127.0.0.1:4747` behind a local reverse proxy. Update the firewall and app port when changing ports. |
| `sessionSigningSecret` | Keep the generated value across restarts and updates. Replacing it invalidates existing sessions. |
| `dataDir` | Use an absolute, persistent path outside `/home` for the systemd service. When moving existing data, stop the service and copy it to the new path before reinstalling. |
| `tlsCertFile`, `tlsKeyFile` | Set both to certificate and key files readable by the service account to serve HTTPS directly. |
| `logLevel` | Set to `debug`, `info`, `warn`, or `error`. |

After changing `dataDir`, run `sudo transonic-server install` to update service
permissions and restart with the new location.

To install a configuration you have prepared yourself, run from the directory
containing the built binary and your JSON file:

```sh
sudo ./transonic-server install --config ./transonic.config.json
```

If a configuration is already installed, this command fails. Add `--replace-config` only
when you intend to overwrite it; save a copy first. A custom configuration needs
its own non-placeholder `sessionSigningSecret`, which you can obtain with
`./transonic-server config generate-secret`.

For access outside a trusted LAN, serve HTTPS using the TLS settings above or a
reverse proxy that forwards HTTP and WebSocket connections, including `/v1/ws`.
Set the app's Connect URL to `https://YOUR_HOST` and its port to the exposed HTTPS
port (usually `443`).

To view recent logs or stop the service:

```sh
sudo journalctl -u transonic-server -n 50 --no-pager
sudo systemctl stop transonic-server
```

Start it again with `sudo systemctl start transonic-server`.

## Run without installing a service

For a foreground run on Linux, start in the directory containing the built binary:

```sh
./transonic-server config print-default > config.json
./transonic-server config generate-secret
```

Edit `config.json`: put the printed secret in `sessionSigningSecret`, set `dataDir`
to a writable directory such as `./data`, and set `listenAddress` to
`0.0.0.0:4747` if other devices need access. The printed default binds to
`127.0.0.1:4747`, which only accepts local connections. For the same origin policy
as the service installer, set `allowedOrigins` to `["*"]`.

Then run:

```sh
./transonic-server serve --config ./config.json
```

Keep that terminal open; **Ctrl+C** stops the server. Follow the health check and
app connection steps above. The configuration and data paths here are independent
of the systemd installation.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| `Exec format error` | Rebuild for the Linux OS architecture in step 2B. |
| Installation fails with a firewalld error | `--open-firewall` needs running firewalld. If you use another firewall, rerun without that flag and configure its TCP rule separately. |
| Service fails to start | Read `sudo journalctl -u transonic-server -n 50 --no-pager`. Check JSON syntax, whether the port is occupied, and whether `dataDir` is an absolute path outside `/home`. |
| Health check succeeds locally but fails on another device | Check `listenAddress`, the server address, and the firewall. A `127.0.0.1` listener is local-only. |
| App shows `Ready` but not `Connected` | Read the message below the Connect settings. Verify that the Connect server can reach the music server URL used by the app and that the music-server credentials are valid. If using a reverse proxy, check WebSocket forwarding. |
| Log says `using ephemeral session signing secret` | Generate a secret, save it as `sessionSigningSecret` in the configuration actually used by the service, and restart. Empty values and values starting with `change-me-` cause sessions to expire on restart. |

## Uninstall

On the Linux server:

```sh
sudo transonic-server service uninstall
```

This stops and disables the service and removes its systemd unit. It keeps the
binary, configuration, and data so you can reinstall later. To delete those too,
add `--remove-binary --remove-config --remove-data`; `--remove-data` deletes
`/var/lib/transonic-server`. A custom `dataDir` and firewall rules must be removed
separately if no longer needed.
