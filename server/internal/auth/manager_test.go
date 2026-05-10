package auth

import (
	"context"
	"strings"
	"testing"
	"time"

	"github.com/Innsbluck-rh/transonic/server/internal/store"
)

func TestIssueAuthenticateRefreshLogout(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()

	manager, err := NewManager(st, []byte("1234567890123456"), time.Hour)
	if err != nil {
		t.Fatal(err)
	}
	issued, err := manager.Issue(ctx, IssueRequest{
		UserKey:           "user",
		DeviceID:          "device",
		UpstreamServerURL: "https://music.example/rest",
		Username:          "demo",
	})
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(issued.AccessToken, ".") {
		t.Fatalf("expected signed token, got %q", issued.AccessToken)
	}

	session, err := manager.Authenticate(ctx, "Bearer "+issued.AccessToken)
	if err != nil {
		t.Fatal(err)
	}
	if session.UserKey != "user" {
		t.Fatalf("unexpected session: %#v", session)
	}

	refreshed, err := manager.Refresh(ctx, issued.AccessToken)
	if err != nil {
		t.Fatal(err)
	}
	if refreshed.AccessToken == issued.AccessToken {
		t.Fatal("expected refresh to rotate token")
	}
	if _, err := manager.Authenticate(ctx, issued.AccessToken); err != ErrInvalidToken {
		t.Fatalf("old token should be invalid, got %v", err)
	}
	if err := manager.Logout(ctx, refreshed.AccessToken); err != nil {
		t.Fatal(err)
	}
	if _, err := manager.Authenticate(ctx, refreshed.AccessToken); err != ErrInvalidToken {
		t.Fatalf("logged out token should be invalid, got %v", err)
	}
}
