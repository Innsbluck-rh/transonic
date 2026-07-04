import { createEffect, onCleanup } from 'solid-js';
import { refreshConnectServerStatus, startConnectStateSync } from '~/features/connect/service';
import { settingsStore } from '~/features/settings/service';
import { sessionStore } from '~/stores/SessionStore';

function ConnectSync() {
  createEffect(() => {
    settingsStore.connect.enabled;
    settingsStore.connect.useSubsonicServerHost;
    settingsStore.connect.connectServerPort;
    settingsStore.connect.connectServerHost;
    settingsStore.connect.allowInsecureConnectServer;
    sessionStore.activeSession?.normalizedServerUrl;
    void refreshConnectServerStatus();
  });

  createEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void startConnectStateSync()
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        unlisten = nextUnlisten;
      })
      .catch((error) => {
        console.error(error);
      });

    onCleanup(() => {
      disposed = true;
      unlisten?.();
    });
  });

  return null;
}

export default ConnectSync;
