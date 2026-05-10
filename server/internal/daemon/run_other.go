//go:build !windows

package daemon

import "context"

func Run(ctx context.Context, _ string, run RunFunc) error {
	return run(ctx)
}
