package main

import (
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/Innsbluck-rh/transonic/server/internal/auth"
	"github.com/Innsbluck-rh/transonic/server/internal/config"
	"github.com/Innsbluck-rh/transonic/server/internal/daemon"
	"github.com/Innsbluck-rh/transonic/server/internal/httpapi"
	"github.com/Innsbluck-rh/transonic/server/internal/opensubsonic"
	"github.com/Innsbluck-rh/transonic/server/internal/realtime"
	"github.com/Innsbluck-rh/transonic/server/internal/store"
)

const serviceName = "transonic-server"

func main() {
	if err := run(os.Args[1:], os.Stdout); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string, out io.Writer) error {
	if len(args) == 0 {
		args = []string{"serve"}
	}
	switch args[0] {
	case "help", "-h", "--help":
		printUsage(out)
		return nil
	case "serve":
		fs := flag.NewFlagSet("serve", flag.ExitOnError)
		fs.SetOutput(out)
		configPath := fs.String("config", config.DefaultPath(), "path to config JSON")
		if err := fs.Parse(args[1:]); err != nil {
			return err
		}
		return daemon.Run(context.Background(), serviceName, func(ctx context.Context) error {
			return serve(ctx, *configPath)
		})
	case "config":
		if len(args) < 2 || args[1] == "help" || args[1] == "-h" || args[1] == "--help" {
			printConfigUsage(out)
			return nil
		}
		switch args[1] {
		case "print-default":
			contents, err := config.PrintableDefault().MarshalPretty()
			if err != nil {
				return err
			}
			fmt.Fprintln(out, string(contents))
			return nil
		case "generate-secret":
			secret, err := config.GenerateSessionSigningSecret()
			if err != nil {
				return err
			}
			fmt.Fprintln(out, secret)
			return nil
		}
		return fmt.Errorf("unknown config command %q\n\n%s", args[1], configUsage())
	case "migrate":
		if len(args) < 2 || args[1] == "help" || args[1] == "-h" || args[1] == "--help" {
			printMigrateUsage(out)
			return nil
		}
		if args[1] == "status" {
			fs := flag.NewFlagSet("migrate status", flag.ExitOnError)
			fs.SetOutput(out)
			configPath := fs.String("config", config.DefaultPath(), "path to config JSON")
			if err := fs.Parse(args[2:]); err != nil {
				return err
			}
			return migrateStatus(*configPath)
		}
		return fmt.Errorf("unknown migrate command %q\n\n%s", args[1], migrateUsage())
	default:
		return fmt.Errorf("unknown command %q\n\n%s", args[0], usage())
	}
}

func printUsage(out io.Writer) {
	fmt.Fprint(out, usage())
}

func usage() string {
	return `transonic-server is the optional broker for transonic Connect.

Usage:
  transonic-server help
  transonic-server serve [--config path]
  transonic-server config <command>
  transonic-server migrate <command>

Commands:
  serve                  Start the HTTP/WebSocket server.
  config print-default   Print a default JSON config.
  config generate-secret Print a random session signing secret.
  migrate status         Print the database schema version.

Common first setup:
  transonic-server config print-default > transonic.config.json
  transonic-server config generate-secret
  transonic-server serve --config transonic.config.json

If serve logs "using ephemeral session signing secret", set
"sessionSigningSecret" in the config to a non-placeholder secret.
`
}

func printConfigUsage(out io.Writer) {
	fmt.Fprint(out, configUsage())
}

func configUsage() string {
	return `Usage:
  transonic-server config print-default
  transonic-server config generate-secret

Commands:
  print-default     Print a default JSON config.
  generate-secret   Print a random value for "sessionSigningSecret".
`
}

func printMigrateUsage(out io.Writer) {
	fmt.Fprint(out, migrateUsage())
}

func migrateUsage() string {
	return `Usage:
  transonic-server migrate status [--config path]

Commands:
  status   Print the database schema version for the configured dataDir.
`
}

func serve(ctx context.Context, configPath string) error {
	cfg, err := config.Load(configPath)
	if err != nil {
		return err
	}
	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: logLevel(cfg.LogLevel)}))
	secret, ephemeral, err := cfg.SessionSecret()
	if err != nil {
		return err
	}
	if ephemeral {
		logger.Warn("using ephemeral session signing secret; sessions will be invalid after restart")
	}

	st, err := store.Open(ctx, cfg.DataDir)
	if err != nil {
		return err
	}
	defer st.Close()

	ttl, err := cfg.TokenTTL()
	if err != nil {
		return err
	}
	authManager, err := auth.NewManager(st, secret, ttl)
	if err != nil {
		return err
	}
	presenceTimeout, err := cfg.DevicePresenceTimeout()
	if err != nil {
		return err
	}
	if err := st.ClearPresence(ctx); err != nil {
		return fmt.Errorf("clear stale presence cache: %w", err)
	}
	api := httpapi.New(
		cfg,
		st,
		authManager,
		opensubsonic.NewClient(nil),
		realtime.NewHub(st, presenceTimeout),
		logger,
	)

	runCtx, stop := signal.NotifyContext(ctx, os.Interrupt, syscall.SIGTERM)
	defer stop()

	errCh := make(chan error, 1)
	go func() {
		logger.Info("transonic server listening", "address", cfg.ListenAddress)
		errCh <- api.ListenAndServe()
	}()

	select {
	case <-runCtx.Done():
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if err := api.Shutdown(shutdownCtx); err != nil {
			return err
		}
		return nil
	case err := <-errCh:
		if errors.Is(err, http.ErrServerClosed) {
			return nil
		}
		return err
	}
}

func migrateStatus(configPath string) error {
	cfg, err := config.Load(configPath)
	if err != nil {
		return err
	}
	st, err := store.Open(context.Background(), cfg.DataDir)
	if err != nil {
		return err
	}
	defer st.Close()
	version, err := st.SchemaVersion(context.Background())
	if err != nil {
		return err
	}
	return json.NewEncoder(os.Stdout).Encode(map[string]string{"schemaVersion": version})
}

func logLevel(value string) slog.Level {
	switch strings.ToLower(value) {
	case "debug":
		return slog.LevelDebug
	case "warn":
		return slog.LevelWarn
	case "error":
		return slog.LevelError
	default:
		return slog.LevelInfo
	}
}
