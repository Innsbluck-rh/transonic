package config

import (
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"net/url"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"time"
)

const (
	defaultListenAddress   = "127.0.0.1:4747"
	defaultSessionTokenTTL = "24h"
	defaultPresenceTimeout = "45s"
	defaultLogLevel        = "info"
)

type Config struct {
	ListenAddress        string   `json:"listenAddress"`
	PublicURL            string   `json:"publicUrl,omitempty"`
	TLSCertFile          string   `json:"tlsCertFile,omitempty"`
	TLSKeyFile           string   `json:"tlsKeyFile,omitempty"`
	DataDir              string   `json:"dataDir"`
	SessionSigningSecret string   `json:"sessionSigningSecret"`
	SessionTokenTTL      string   `json:"sessionTokenTtl"`
	AllowedOrigins       []string `json:"allowedOrigins"`
	LogLevel             string   `json:"logLevel"`
	UpstreamAllowlist    []string `json:"upstreamAllowlist,omitempty"`
	PresenceTimeout      string   `json:"presenceTimeout"`
}

func Default() Config {
	return Config{
		ListenAddress:        defaultListenAddress,
		DataDir:              DefaultDataDir(),
		SessionSigningSecret: "",
		SessionTokenTTL:      defaultSessionTokenTTL,
		AllowedOrigins:       []string{"http://127.0.0.1:*", "http://localhost:*", "tauri://localhost"},
		LogLevel:             defaultLogLevel,
		PresenceTimeout:      defaultPresenceTimeout,
	}
}

func PrintableDefault() Config {
	cfg := Default()
	cfg.SessionSigningSecret = "change-me-generate-with-transonic-server-config-print-default"
	return cfg
}

func DefaultPath() string {
	switch runtime.GOOS {
	case "windows":
		root := os.Getenv("ProgramData")
		if root == "" {
			root = `C:\ProgramData`
		}
		return filepath.Join(root, "transonic-server", "config.json")
	case "darwin":
		return "/Library/Application Support/transonic-server/config.json"
	default:
		return "/etc/transonic-server/config.json"
	}
}

func DefaultDataDir() string {
	switch runtime.GOOS {
	case "windows":
		root := os.Getenv("ProgramData")
		if root == "" {
			root = `C:\ProgramData`
		}
		return filepath.Join(root, "transonic-server", "data")
	case "darwin":
		return "/Library/Application Support/transonic-server"
	default:
		return "/var/lib/transonic-server"
	}
}

func Load(path string) (Config, error) {
	if path == "" {
		path = DefaultPath()
	}

	cfg := Default()
	contents, err := os.ReadFile(path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			if path != DefaultPath() {
				cfg.DataDir = filepath.Dir(path)
			}
			return cfg, nil
		}
		return Config{}, fmt.Errorf("read config: %w", err)
	}

	if err := json.Unmarshal(contents, &cfg); err != nil {
		return Config{}, fmt.Errorf("parse config: %w", err)
	}
	cfg.applyDefaults()
	return cfg, cfg.Validate()
}

func (c *Config) applyDefaults() {
	if c.ListenAddress == "" {
		c.ListenAddress = defaultListenAddress
	}
	if c.DataDir == "" {
		c.DataDir = DefaultDataDir()
	}
	if c.SessionTokenTTL == "" {
		c.SessionTokenTTL = defaultSessionTokenTTL
	}
	if c.PresenceTimeout == "" {
		c.PresenceTimeout = defaultPresenceTimeout
	}
	if c.LogLevel == "" {
		c.LogLevel = defaultLogLevel
	}
}

func (c Config) Validate() error {
	if strings.TrimSpace(c.ListenAddress) == "" {
		return errors.New("listenAddress is required")
	}
	if strings.TrimSpace(c.DataDir) == "" {
		return errors.New("dataDir is required")
	}
	if _, err := c.TokenTTL(); err != nil {
		return err
	}
	if _, err := c.DevicePresenceTimeout(); err != nil {
		return err
	}
	if (c.TLSCertFile == "") != (c.TLSKeyFile == "") {
		return errors.New("tlsCertFile and tlsKeyFile must be set together")
	}
	if c.PublicURL != "" {
		parsed, err := url.Parse(c.PublicURL)
		if err != nil || parsed.Scheme == "" || parsed.Host == "" {
			return errors.New("publicUrl must be an absolute URL")
		}
	}
	for _, upstream := range c.UpstreamAllowlist {
		parsed, err := url.Parse(upstream)
		if err != nil || parsed.Scheme == "" || parsed.Host == "" {
			return fmt.Errorf("upstreamAllowlist contains invalid URL %q", upstream)
		}
	}
	return nil
}

func (c Config) TokenTTL() (time.Duration, error) {
	ttl, err := time.ParseDuration(c.SessionTokenTTL)
	if err != nil {
		return 0, fmt.Errorf("sessionTokenTtl is invalid: %w", err)
	}
	if ttl <= 0 {
		return 0, errors.New("sessionTokenTtl must be positive")
	}
	return ttl, nil
}

func (c Config) DevicePresenceTimeout() (time.Duration, error) {
	timeout, err := time.ParseDuration(c.PresenceTimeout)
	if err != nil {
		return 0, fmt.Errorf("presenceTimeout is invalid: %w", err)
	}
	if timeout <= 0 {
		return 0, errors.New("presenceTimeout must be positive")
	}
	return timeout, nil
}

func (c Config) SessionSecret() ([]byte, bool, error) {
	if c.SessionSigningSecret != "" && !strings.HasPrefix(c.SessionSigningSecret, "change-me-") {
		return []byte(c.SessionSigningSecret), false, nil
	}

	secret, err := GenerateSessionSigningSecret()
	if err != nil {
		return nil, false, err
	}
	return []byte(secret), true, nil
}

func GenerateSessionSigningSecret() (string, error) {
	var randomBytes [32]byte
	if _, err := rand.Read(randomBytes[:]); err != nil {
		return "", fmt.Errorf("generate session signing secret: %w", err)
	}
	return base64.RawURLEncoding.EncodeToString(randomBytes[:]), nil
}

func (c Config) UpstreamAllowed(normalized string) bool {
	if len(c.UpstreamAllowlist) == 0 {
		return true
	}
	parsed, err := url.Parse(normalized)
	if err != nil {
		return false
	}
	for _, allowed := range c.UpstreamAllowlist {
		allowedURL, err := url.Parse(allowed)
		if err != nil {
			continue
		}
		if strings.EqualFold(parsed.Scheme, allowedURL.Scheme) && strings.EqualFold(parsed.Host, allowedURL.Host) {
			return true
		}
	}
	return false
}

func (c Config) MarshalPretty() ([]byte, error) {
	return json.MarshalIndent(c, "", "  ")
}
