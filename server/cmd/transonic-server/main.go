package main

import (
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
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
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 {
		args = []string{"serve"}
	}
	switch args[0] {
	case "serve":
		fs := flag.NewFlagSet("serve", flag.ExitOnError)
		configPath := fs.String("config", config.DefaultPath(), "path to config JSON")
		if err := fs.Parse(args[1:]); err != nil {
			return err
		}
		return daemon.Run(context.Background(), serviceName, func(ctx context.Context) error {
			return serve(ctx, *configPath)
		})
	case "config":
		if len(args) >= 2 && args[1] == "print-default" {
			contents, err := config.PrintableDefault().MarshalPretty()
			if err != nil {
				return err
			}
			fmt.Println(string(contents))
			return nil
		}
		return errors.New("usage: transonic-server config print-default")
	case "migrate":
		if len(args) >= 2 && args[1] == "status" {
			fs := flag.NewFlagSet("migrate status", flag.ExitOnError)
			configPath := fs.String("config", config.DefaultPath(), "path to config JSON")
			if err := fs.Parse(args[2:]); err != nil {
				return err
			}
			return migrateStatus(*configPath)
		}
		return errors.New("usage: transonic-server migrate status [--config path]")
	default:
		return fmt.Errorf("unknown command %q", args[0])
	}
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
