package daemon

import "context"

type RunFunc func(context.Context) error
