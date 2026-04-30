//go:build windows

package daemon

import (
	"context"

	"golang.org/x/sys/windows/svc"
)

func Run(ctx context.Context, name string, run RunFunc) error {
	isService, err := svc.IsWindowsService()
	if err != nil || !isService {
		return run(ctx)
	}
	return svc.Run(name, &windowsService{run: run})
}

type windowsService struct {
	run RunFunc
}

func (s *windowsService) Execute(args []string, requests <-chan svc.ChangeRequest, changes chan<- svc.Status) (bool, uint32) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	changes <- svc.Status{State: svc.StartPending}
	errCh := make(chan error, 1)
	go func() {
		errCh <- s.run(ctx)
	}()
	changes <- svc.Status{State: svc.Running, Accepts: svc.AcceptStop | svc.AcceptShutdown}

	for {
		select {
		case request := <-requests:
			switch request.Cmd {
			case svc.Interrogate:
				changes <- request.CurrentStatus
			case svc.Stop, svc.Shutdown:
				changes <- svc.Status{State: svc.StopPending}
				cancel()
				<-errCh
				return false, 0
			default:
			}
		case <-errCh:
			return false, 1
		}
	}
}
