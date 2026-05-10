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
		"transonic-server serve [--config path]",
		"config generate-secret",
		"using ephemeral session signing secret",
	} {
		if !strings.Contains(got, want) {
			t.Fatalf("help output should contain %q, got:\n%s", want, got)
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
