package main

import (
	"bytes"
	"errors"
	"flag"
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"github.com/Innsbluck-rh/transonic/server/internal/config"
)

const (
	linuxServiceUser      = "transonic-server"
	linuxServiceGroup     = "transonic-server"
	linuxServiceName      = "transonic-server"
	linuxInstallBinPath   = "/usr/bin/transonic-server"
	linuxInstallConfigDir = "/etc/transonic-server"
	linuxInstallConfig    = "/etc/transonic-server/config.json"
	linuxInstallDataDir   = "/var/lib/transonic-server"
	linuxInstallUnitPath  = "/etc/systemd/system/transonic-server.service"
)

type serviceInstallOptions struct {
	configSource string
	listen       string
	replaceCfg   bool
	noEnable     bool
	noStart      bool
	openFirewall bool
	firewallZone string
}

type serviceUninstallOptions struct {
	removeBinary bool
	removeConfig bool
	removeData   bool
}

func runInstall(args []string, out io.Writer) error {
	fs := flag.NewFlagSet("install", flag.ExitOnError)
	fs.SetOutput(out)
	opts := serviceInstallOptions{}
	fs.StringVar(&opts.configSource, "config", "", "existing config JSON to install; generated when omitted")
	fs.StringVar(&opts.listen, "listen", "0.0.0.0:4747", "listenAddress for generated config")
	fs.BoolVar(&opts.replaceCfg, "replace-config", false, "replace an existing installed config")
	fs.BoolVar(&opts.noEnable, "no-enable", false, "install files without enabling the systemd service")
	fs.BoolVar(&opts.noStart, "no-start", false, "install files without starting or restarting the systemd service")
	fs.BoolVar(&opts.openFirewall, "open-firewall", false, "open the server TCP port with firewalld")
	fs.StringVar(&opts.firewallZone, "firewall-zone", "", "firewalld zone to update; auto-detected when omitted")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if fs.NArg() != 0 {
		return fmt.Errorf("unexpected install argument %q\n\n%s", fs.Arg(0), installUsage())
	}
	return installLinuxService(opts, out)
}

func runService(args []string, out io.Writer) error {
	if len(args) == 0 || args[0] == "help" || args[0] == "-h" || args[0] == "--help" {
		printServiceUsage(out)
		return nil
	}
	switch args[0] {
	case "install":
		return runInstall(args[1:], out)
	case "uninstall":
		return runServiceUninstall(args[1:], out)
	case "status":
		return runServiceStatus(args[1:], out)
	default:
		return fmt.Errorf("unknown service command %q\n\n%s", args[0], serviceUsage())
	}
}

func runServiceUninstall(args []string, out io.Writer) error {
	fs := flag.NewFlagSet("service uninstall", flag.ExitOnError)
	fs.SetOutput(out)
	opts := serviceUninstallOptions{}
	fs.BoolVar(&opts.removeBinary, "remove-binary", false, "remove /usr/bin/transonic-server")
	fs.BoolVar(&opts.removeConfig, "remove-config", false, "remove /etc/transonic-server")
	fs.BoolVar(&opts.removeData, "remove-data", false, "remove /var/lib/transonic-server")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if fs.NArg() != 0 {
		return fmt.Errorf("unexpected service uninstall argument %q\n\n%s", fs.Arg(0), serviceUsage())
	}
	return uninstallLinuxService(opts, out)
}

func runServiceStatus(args []string, out io.Writer) error {
	fs := flag.NewFlagSet("service status", flag.ExitOnError)
	fs.SetOutput(out)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if fs.NArg() != 0 {
		return fmt.Errorf("unexpected service status argument %q\n\n%s", fs.Arg(0), serviceUsage())
	}
	if runtime.GOOS != "linux" {
		return errors.New("service status is only supported on Linux")
	}
	if _, err := exec.LookPath("systemctl"); err != nil {
		return errors.New("systemctl was not found")
	}
	return runCmd(out, "systemctl", "status", linuxServiceName, "--no-pager")
}

func printServiceUsage(out io.Writer) {
	fmt.Fprint(out, serviceUsage())
}

func installUsage() string {
	return `Usage:
  transonic-server install [--config path] [--replace-config] [--open-firewall]

Installs the current binary as a Linux systemd service. When --config is omitted,
the installer creates /etc/transonic-server/config.json with a generated
sessionSigningSecret, listenAddress 0.0.0.0:4747, and dataDir
/var/lib/transonic-server.
`
}

func serviceUsage() string {
	return `Usage:
  transonic-server service install [options]
  transonic-server service uninstall [options]
  transonic-server service status

Commands:
  install     Install/update the Linux systemd service.
  uninstall   Stop/disable the service and remove its systemd unit.
  status      Print systemctl status for transonic-server.

Install options:
  --config path       Existing config JSON to install; generated when omitted.
  --listen address    listenAddress for generated config. Default: 0.0.0.0:4747.
  --replace-config    Replace an existing installed config.
  --no-enable         Do not enable the systemd service.
  --no-start          Do not start or restart the systemd service.
  --open-firewall     Add the configured TCP port to firewalld.
  --firewall-zone z   firewalld zone to update. Auto-detects tailscale0 first.

Uninstall options:
  --remove-binary     Remove /usr/bin/transonic-server.
  --remove-config     Remove /etc/transonic-server.
  --remove-data       Remove /var/lib/transonic-server.
`
}

func installLinuxService(opts serviceInstallOptions, out io.Writer) error {
	if err := requireLinuxRoot("install"); err != nil {
		return err
	}
	if _, err := exec.LookPath("systemctl"); err != nil {
		return errors.New("systemctl was not found; systemd is required for install")
	}

	self, err := os.Executable()
	if err != nil {
		return fmt.Errorf("resolve current executable: %w", err)
	}
	self, err = filepath.EvalSymlinks(self)
	if err != nil {
		return fmt.Errorf("resolve current executable symlink: %w", err)
	}

	if err := ensureLinuxServiceAccount(out); err != nil {
		return err
	}
	if err := ensureInstallDirs(out); err != nil {
		return err
	}
	if err := copyExecutable(self, linuxInstallBinPath); err != nil {
		return err
	}
	cfg, configAction, err := installConfig(opts)
	if err != nil {
		return err
	}
	if err := ensureDataDir(cfg.DataDir, out); err != nil {
		return err
	}
	if err := writeTextFile(linuxInstallUnitPath, serviceUnit(cfg.DataDir), 0o644); err != nil {
		return fmt.Errorf("write systemd unit %s: %w", linuxInstallUnitPath, err)
	}
	if err := runCmd(out, "systemctl", "daemon-reload"); err != nil {
		return err
	}
	if opts.openFirewall {
		if err := openFirewallPort(cfg.ListenAddress, opts.firewallZone, out); err != nil {
			return err
		}
	}
	if !opts.noEnable {
		if err := runCmd(out, "systemctl", "enable", linuxServiceName); err != nil {
			return err
		}
	}
	if !opts.noStart {
		if err := runCmd(out, "systemctl", "restart", linuxServiceName); err != nil {
			return err
		}
		if err := waitForServiceActive(out); err != nil {
			return err
		}
	}

	enabled := "not requested"
	if !opts.noEnable {
		enabled = strings.TrimSpace(commandOutput("systemctl", "is-enabled", linuxServiceName))
	}
	active := "not requested"
	if !opts.noStart {
		active = strings.TrimSpace(commandOutput("systemctl", "is-active", linuxServiceName))
	}
	fmt.Fprintln(out, "transonic-server service installed")
	fmt.Fprintf(out, "  binary: %s\n", linuxInstallBinPath)
	fmt.Fprintf(out, "  config: %s (%s)\n", linuxInstallConfig, configAction)
	fmt.Fprintf(out, "  data:   %s\n", cfg.DataDir)
	fmt.Fprintf(out, "  unit:   %s\n", linuxInstallUnitPath)
	fmt.Fprintf(out, "  listen: %s\n", cfg.ListenAddress)
	fmt.Fprintf(out, "  enable: %s\n", enabled)
	fmt.Fprintf(out, "  active: %s\n", active)
	return nil
}

func uninstallLinuxService(opts serviceUninstallOptions, out io.Writer) error {
	if err := requireLinuxRoot("service uninstall"); err != nil {
		return err
	}
	if _, err := exec.LookPath("systemctl"); err != nil {
		return errors.New("systemctl was not found; systemd is required for uninstall")
	}
	_ = runCmd(out, "systemctl", "stop", linuxServiceName)
	_ = runCmd(out, "systemctl", "disable", linuxServiceName)
	if err := os.Remove(linuxInstallUnitPath); err != nil && !errors.Is(err, os.ErrNotExist) {
		return fmt.Errorf("remove systemd unit %s: %w", linuxInstallUnitPath, err)
	}
	if opts.removeBinary {
		if err := os.Remove(linuxInstallBinPath); err != nil && !errors.Is(err, os.ErrNotExist) {
			return fmt.Errorf("remove binary %s: %w", linuxInstallBinPath, err)
		}
	}
	if opts.removeConfig {
		if err := os.RemoveAll(linuxInstallConfigDir); err != nil {
			return fmt.Errorf("remove config directory %s: %w", linuxInstallConfigDir, err)
		}
	}
	if opts.removeData {
		if err := os.RemoveAll(linuxInstallDataDir); err != nil {
			return fmt.Errorf("remove data directory %s: %w", linuxInstallDataDir, err)
		}
	}
	if err := runCmd(out, "systemctl", "daemon-reload"); err != nil {
		return err
	}
	fmt.Fprintln(out, "transonic-server service uninstalled")
	return nil
}

func requireLinuxRoot(action string) error {
	if runtime.GOOS != "linux" {
		return fmt.Errorf("%s is only supported on Linux", action)
	}
	if os.Geteuid() != 0 {
		return fmt.Errorf("%s must be run as root; retry with sudo", action)
	}
	return nil
}

func ensureLinuxServiceAccount(out io.Writer) error {
	if err := exec.Command("getent", "group", linuxServiceGroup).Run(); err != nil {
		if err := runCmd(out, "groupadd", "--system", linuxServiceGroup); err != nil {
			return err
		}
	}
	if err := exec.Command("id", "-u", linuxServiceUser).Run(); err != nil {
		shell := firstExistingPath("/usr/sbin/nologin", "/sbin/nologin", "/bin/false")
		if err := runCmd(
			out,
			"useradd",
			"--system",
			"--no-create-home",
			"--gid",
			linuxServiceGroup,
			"--home-dir",
			linuxInstallDataDir,
			"--shell",
			shell,
			linuxServiceUser,
		); err != nil {
			return err
		}
	}
	return nil
}

func ensureInstallDirs(out io.Writer) error {
	if err := os.MkdirAll(linuxInstallConfigDir, 0o750); err != nil {
		return fmt.Errorf("create directory %s: %w", linuxInstallConfigDir, err)
	}
	if err := runCmd(out, "chown", "root:"+linuxServiceGroup, linuxInstallConfigDir); err != nil {
		return err
	}
	if err := os.Chmod(linuxInstallConfigDir, 0o750); err != nil {
		return fmt.Errorf("chmod %s: %w", linuxInstallConfigDir, err)
	}
	return nil
}

func ensureDataDir(path string, out io.Writer) error {
	clean := filepath.Clean(path)
	if !filepath.IsAbs(clean) {
		return fmt.Errorf("dataDir must be absolute for systemd install: %s", path)
	}
	if clean == "/home" || strings.HasPrefix(clean, "/home/") {
		return fmt.Errorf("dataDir %s is under /home, but the systemd unit uses ProtectHome=true; use %s or another non-home path", clean, linuxInstallDataDir)
	}
	if err := os.MkdirAll(clean, 0o750); err != nil {
		return fmt.Errorf("create data directory %s: %w", clean, err)
	}
	if err := runCmd(out, "chown", "-R", linuxServiceUser+":"+linuxServiceGroup, clean); err != nil {
		return err
	}
	if err := os.Chmod(clean, 0o750); err != nil {
		return fmt.Errorf("chmod %s: %w", clean, err)
	}
	return nil
}

func copyExecutable(source, dest string) error {
	same, err := samePath(source, dest)
	if err != nil {
		return err
	}
	if same {
		return os.Chmod(dest, 0o755)
	}
	return copyFileAtomic(source, dest, 0o755)
}

func installConfig(opts serviceInstallOptions) (config.Config, string, error) {
	if _, err := os.Stat(linuxInstallConfig); err == nil && !opts.replaceCfg {
		if opts.configSource != "" {
			return config.Config{}, "", fmt.Errorf("%s already exists; pass --replace-config to install %s", linuxInstallConfig, opts.configSource)
		}
		cfg, err := config.Load(linuxInstallConfig)
		if err != nil {
			return config.Config{}, "", err
		}
		return cfg, "preserved existing", nil
	} else if err != nil && !errors.Is(err, os.ErrNotExist) {
		return config.Config{}, "", fmt.Errorf("stat config %s: %w", linuxInstallConfig, err)
	}

	if opts.configSource != "" {
		source, err := filepath.Abs(opts.configSource)
		if err != nil {
			return config.Config{}, "", fmt.Errorf("resolve config source %s: %w", opts.configSource, err)
		}
		if same, err := samePath(source, linuxInstallConfig); err != nil {
			return config.Config{}, "", err
		} else if !same {
			if err := copyFileAtomic(source, linuxInstallConfig, 0o640); err != nil {
				return config.Config{}, "", err
			}
		}
		if err := runCmd(io.Discard, "chown", "root:"+linuxServiceGroup, linuxInstallConfig); err != nil {
			return config.Config{}, "", err
		}
		if err := os.Chmod(linuxInstallConfig, 0o640); err != nil {
			return config.Config{}, "", fmt.Errorf("chmod %s: %w", linuxInstallConfig, err)
		}
		cfg, err := config.Load(linuxInstallConfig)
		if err != nil {
			return config.Config{}, "", err
		}
		return cfg, "copied", nil
	}

	contents, cfg, err := generatedInstallConfig(opts.listen)
	if err != nil {
		return config.Config{}, "", err
	}
	if err := writeTextFile(linuxInstallConfig, string(contents), 0o640); err != nil {
		return config.Config{}, "", fmt.Errorf("write config %s: %w", linuxInstallConfig, err)
	}
	if err := runCmd(io.Discard, "chown", "root:"+linuxServiceGroup, linuxInstallConfig); err != nil {
		return config.Config{}, "", err
	}
	if err := os.Chmod(linuxInstallConfig, 0o640); err != nil {
		return config.Config{}, "", fmt.Errorf("chmod %s: %w", linuxInstallConfig, err)
	}
	return cfg, "generated", nil
}

func generatedInstallConfig(listen string) ([]byte, config.Config, error) {
	cfg := config.Default()
	cfg.ListenAddress = listen
	cfg.DataDir = linuxInstallDataDir
	cfg.AllowedOrigins = []string{"*"}
	secret, err := config.GenerateSessionSigningSecret()
	if err != nil {
		return nil, config.Config{}, err
	}
	cfg.SessionSigningSecret = secret
	if err := cfg.Validate(); err != nil {
		return nil, config.Config{}, err
	}
	contents, err := cfg.MarshalPretty()
	if err != nil {
		return nil, config.Config{}, err
	}
	return contents, cfg, nil
}

func serviceUnit(dataDir string) string {
	return fmt.Sprintf(`[Unit]
Description=transonic server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=%s
Group=%s
ExecStart=%s serve --config %s
Restart=on-failure
RestartSec=5s
StateDirectory=transonic-server
ConfigurationDirectory=transonic-server
StateDirectoryMode=0750
ConfigurationDirectoryMode=0750
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=%s

[Install]
WantedBy=multi-user.target
`, linuxServiceUser, linuxServiceGroup, linuxInstallBinPath, linuxInstallConfig, dataDir)
}

func openFirewallPort(listenAddress, zone string, out io.Writer) error {
	if _, err := exec.LookPath("firewall-cmd"); err != nil {
		return errors.New("--open-firewall was requested, but firewall-cmd was not found")
	}
	if err := exec.Command("firewall-cmd", "--state").Run(); err != nil {
		return errors.New("--open-firewall was requested, but firewalld is not running")
	}
	port := listenPort(listenAddress)
	if port == "" {
		return fmt.Errorf("cannot determine TCP port from listenAddress %q", listenAddress)
	}
	if zone == "" {
		zone = strings.TrimSpace(commandOutput("firewall-cmd", "--get-zone-of-interface=tailscale0"))
	}
	if zone == "" {
		zone = strings.TrimSpace(commandOutput("firewall-cmd", "--get-default-zone"))
	}
	if zone == "" {
		return errors.New("could not determine firewalld zone; pass --firewall-zone")
	}
	if err := runCmd(out, "firewall-cmd", "--permanent", "--zone="+zone, "--add-port="+port+"/tcp"); err != nil {
		return err
	}
	if err := runCmd(out, "firewall-cmd", "--reload"); err != nil {
		return err
	}
	fmt.Fprintf(out, "firewall: opened %s/tcp in zone %s\n", port, zone)
	return nil
}

func waitForServiceActive(out io.Writer) error {
	deadline := time.Now().Add(5 * time.Second)
	var lastState string
	var activeSince time.Time
	for time.Now().Before(deadline) {
		lastState = strings.TrimSpace(commandOutput("systemctl", "is-active", linuxServiceName))
		if lastState == "active" {
			if activeSince.IsZero() {
				activeSince = time.Now()
			}
			if time.Since(activeSince) >= 2*time.Second {
				return nil
			}
		} else {
			activeSince = time.Time{}
		}
		if lastState == "failed" || lastState == "inactive" {
			break
		}
		time.Sleep(250 * time.Millisecond)
	}

	var status bytes.Buffer
	statusCmd := exec.Command("systemctl", "status", linuxServiceName, "--no-pager")
	statusCmd.Stdout = &status
	statusCmd.Stderr = &status
	_ = statusCmd.Run()

	var journal bytes.Buffer
	journalCmd := exec.Command("journalctl", "-u", linuxServiceName, "-n", "40", "--no-pager")
	journalCmd.Stdout = &journal
	journalCmd.Stderr = &journal
	_ = journalCmd.Run()

	fmt.Fprintln(out, strings.TrimSpace(status.String()))
	if journal.Len() > 0 {
		fmt.Fprintln(out, strings.TrimSpace(journal.String()))
	}
	if lastState == "" {
		lastState = "unknown"
	}
	return fmt.Errorf("transonic-server service did not stay active after restart; state=%s", lastState)
}

func listenPort(listenAddress string) string {
	_, port, err := net.SplitHostPort(strings.TrimSpace(listenAddress))
	if err != nil || port == "" {
		return ""
	}
	return port
}

func firstExistingPath(paths ...string) string {
	for _, path := range paths {
		if _, err := os.Stat(path); err == nil {
			return path
		}
	}
	return paths[len(paths)-1]
}

func samePath(left, right string) (bool, error) {
	leftAbs, err := filepath.Abs(left)
	if err != nil {
		return false, fmt.Errorf("resolve path %s: %w", left, err)
	}
	rightAbs, err := filepath.Abs(right)
	if err != nil {
		return false, fmt.Errorf("resolve path %s: %w", right, err)
	}
	leftEval, leftErr := filepath.EvalSymlinks(leftAbs)
	rightEval, rightErr := filepath.EvalSymlinks(rightAbs)
	if leftErr == nil {
		leftAbs = leftEval
	}
	if rightErr == nil {
		rightAbs = rightEval
	}
	return leftAbs == rightAbs, nil
}

func copyFileAtomic(source, dest string, mode os.FileMode) error {
	src, err := os.Open(source)
	if err != nil {
		return fmt.Errorf("open %s: %w", source, err)
	}
	defer src.Close()

	if err := os.MkdirAll(filepath.Dir(dest), 0o755); err != nil {
		return fmt.Errorf("create parent directory for %s: %w", dest, err)
	}
	tmp, err := os.CreateTemp(filepath.Dir(dest), "."+filepath.Base(dest)+".tmp-*")
	if err != nil {
		return fmt.Errorf("create temp file for %s: %w", dest, err)
	}
	tmpPath := tmp.Name()
	removeTmp := true
	defer func() {
		if removeTmp {
			_ = os.Remove(tmpPath)
		}
	}()
	if _, err := io.Copy(tmp, src); err != nil {
		_ = tmp.Close()
		return fmt.Errorf("copy %s to %s: %w", source, dest, err)
	}
	if err := tmp.Chmod(mode); err != nil {
		_ = tmp.Close()
		return fmt.Errorf("chmod temp file for %s: %w", dest, err)
	}
	if err := tmp.Close(); err != nil {
		return fmt.Errorf("close temp file for %s: %w", dest, err)
	}
	if err := os.Rename(tmpPath, dest); err != nil {
		return fmt.Errorf("replace %s: %w", dest, err)
	}
	removeTmp = false
	return nil
}

func writeTextFile(path, contents string, mode os.FileMode) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	tmp, err := os.CreateTemp(filepath.Dir(path), "."+filepath.Base(path)+".tmp-*")
	if err != nil {
		return err
	}
	tmpPath := tmp.Name()
	removeTmp := true
	defer func() {
		if removeTmp {
			_ = os.Remove(tmpPath)
		}
	}()
	if _, err := tmp.WriteString(contents); err != nil {
		_ = tmp.Close()
		return err
	}
	if err := tmp.Chmod(mode); err != nil {
		_ = tmp.Close()
		return err
	}
	if err := tmp.Close(); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, path); err != nil {
		return err
	}
	removeTmp = false
	return nil
}

func runCmd(out io.Writer, name string, args ...string) error {
	cmd := exec.Command(name, args...)
	var stderr bytes.Buffer
	cmd.Stdout = out
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		detail := strings.TrimSpace(stderr.String())
		if detail == "" {
			return fmt.Errorf("%s %s failed: %w", name, strings.Join(args, " "), err)
		}
		return fmt.Errorf("%s %s failed: %w: %s", name, strings.Join(args, " "), err, detail)
	}
	return nil
}

func commandOutput(name string, args ...string) string {
	output, err := exec.Command(name, args...).Output()
	if err != nil {
		return ""
	}
	return string(output)
}
