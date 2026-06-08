package main

import (
	"bytes"
	"strings"
	"testing"
)

func TestRunHelpPrintsTopLevelUsage(t *testing.T) {
	var out bytes.Buffer
	if err := run([]string{"help"}, &out); err != nil {
		t.Fatalf("run returned error: %v", err)
	}
	got := out.String()
	for _, want := range []string{
		"transonic-server install [options]",
		"transonic-server serve [--config path]",
		"service install",
		"config generate-secret",
		"using ephemeral session signing secret",
	} {
		if !strings.Contains(got, want) {
			t.Fatalf("help output should contain %q, got:\n%s", want, got)
		}
	}
}

func TestRunServiceHelpPrintsUsage(t *testing.T) {
	var out bytes.Buffer
	if err := run([]string{"service"}, &out); err != nil {
		t.Fatalf("run returned error: %v", err)
	}
	got := out.String()
	for _, want := range []string{
		"transonic-server service install [options]",
		"--open-firewall",
		"--remove-data",
	} {
		if !strings.Contains(got, want) {
			t.Fatalf("service help output should contain %q, got:\n%s", want, got)
		}
	}
}

func TestRunConfigGenerateSecretPrintsSecret(t *testing.T) {
	var out bytes.Buffer
	if err := run([]string{"config", "generate-secret"}, &out); err != nil {
		t.Fatalf("run returned error: %v", err)
	}
	if strings.TrimSpace(out.String()) == "" {
		t.Fatal("expected generated secret")
	}
}

func TestRunUnknownCommandIncludesUsage(t *testing.T) {
	var out bytes.Buffer
	err := run([]string{"wat"}, &out)
	if err == nil {
		t.Fatal("expected unknown command error")
	}
	if !strings.Contains(err.Error(), "unknown command") || !strings.Contains(err.Error(), "Usage:") {
		t.Fatalf("expected error with usage, got %v", err)
	}
}

func TestGeneratedInstallConfigUsesServiceDefaults(t *testing.T) {
	contents, cfg, err := generatedInstallConfig("0.0.0.0:4747")
	if err != nil {
		t.Fatalf("generatedInstallConfig returned error: %v", err)
	}
	if cfg.ListenAddress != "0.0.0.0:4747" {
		t.Fatalf("unexpected listenAddress %q", cfg.ListenAddress)
	}
	if cfg.DataDir != linuxInstallDataDir {
		t.Fatalf("unexpected dataDir %q", cfg.DataDir)
	}
	if cfg.SessionSigningSecret == "" || strings.HasPrefix(cfg.SessionSigningSecret, "change-me-") {
		t.Fatalf("expected generated non-placeholder signing secret")
	}
	if !strings.Contains(string(contents), `"allowedOrigins": [`) || !strings.Contains(string(contents), `"*"`) {
		t.Fatalf("expected generated config to allow origins for dev deployment, got:\n%s", string(contents))
	}
}

func TestListenPortParsesIPv4AndIPv6(t *testing.T) {
	for input, want := range map[string]string{
		"0.0.0.0:4747":       "4747",
		"100.113.216.2:4747": "4747",
		"[::]:4747":          "4747",
		":4747":              "4747",
		"4747":               "",
	} {
		if got := listenPort(input); got != want {
			t.Fatalf("listenPort(%q) = %q, want %q", input, got, want)
		}
	}
}

func TestServiceUnitUsesRestrictedDirectoryModes(t *testing.T) {
	unit := serviceUnit(linuxInstallDataDir)
	for _, want := range []string{
		"StateDirectoryMode=0750",
		"ConfigurationDirectoryMode=0750",
		"ReadWritePaths=/var/lib/transonic-server",
	} {
		if !strings.Contains(unit, want) {
			t.Fatalf("service unit should contain %q, got:\n%s", want, unit)
		}
	}
}
