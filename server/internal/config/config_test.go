package config

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadMissingUsesDefaults(t *testing.T) {
	dir := t.TempDir()
	cfg, err := Load(filepath.Join(dir, "missing.json"))
	if err != nil {
		t.Fatalf("Load returned error: %v", err)
	}
	if cfg.ListenAddress == "" || cfg.DataDir == "" {
		t.Fatalf("expected defaults, got %#v", cfg)
	}
	if cfg.DataDir != dir {
		t.Fatalf("expected missing explicit config to use config directory as data dir, got %s", cfg.DataDir)
	}
}

func TestLoadParsesDurationsAndAllowlist(t *testing.T) {
	path := filepath.Join(t.TempDir(), "config.json")
	if err := os.WriteFile(path, []byte(`{
  "listenAddress": "127.0.0.1:4747",
  "dataDir": "data",
  "sessionSigningSecret": "secret",
  "sessionTokenTtl": "1h",
  "presenceTimeout": "30s",
  "upstreamAllowlist": ["https://music.example"]
}`), 0o600); err != nil {
		t.Fatal(err)
	}

	cfg, err := Load(path)
	if err != nil {
		t.Fatalf("Load returned error: %v", err)
	}
	if _, err := cfg.TokenTTL(); err != nil {
		t.Fatalf("TokenTTL returned error: %v", err)
	}
	if !cfg.UpstreamAllowed("https://music.example/rest") {
		t.Fatal("expected allowlisted upstream")
	}
	if cfg.UpstreamAllowed("https://other.example/rest") {
		t.Fatal("expected non-allowlisted upstream to be rejected")
	}
}

func TestInvalidDurationFails(t *testing.T) {
	path := filepath.Join(t.TempDir(), "config.json")
	if err := os.WriteFile(path, []byte(`{"sessionTokenTtl":"never"}`), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := Load(path); err == nil {
		t.Fatal("expected invalid duration error")
	}
}
