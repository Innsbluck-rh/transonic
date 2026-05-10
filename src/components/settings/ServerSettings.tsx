import { createSignal, Show } from 'solid-js';
import { commands } from '~/bindings';
import { loadBootstrapToStore } from '~/features/session/service';
import { sessionStore } from '~/stores/SessionStore';
import SettingSection from './SettingSection';

function ServerSettings() {
  const activeSession = () => sessionStore.activeSession;
  const [backupMessage, setBackupMessage] = createSignal<string | null>(null);
  const [backupBusy, setBackupBusy] = createSignal(false);

  const exportBackup = async () => {
    setBackupBusy(true);
    setBackupMessage(null);
    try {
      const result = await commands.exportServerBackup();
      if (result.status === 'error') {
        setBackupMessage(result.error);
        return;
      }
      setBackupMessage(`Exported ${result.data.profileCount} profile(s): ${result.data.path}`);
    } catch (error) {
      setBackupMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBackupBusy(false);
    }
  };

  const importBackup = async () => {
    setBackupBusy(true);
    setBackupMessage(null);
    try {
      const result = await commands.importServerBackup({ replaceExisting: true });
      if (result.status === 'error') {
        setBackupMessage(result.error);
        return;
      }
      const bootstrapResult = await commands.bootstrapAppState();
      if (bootstrapResult.status === 'error') {
        setBackupMessage(bootstrapResult.error);
        return;
      }
      await loadBootstrapToStore(bootstrapResult.data);
      setBackupMessage(`Imported ${result.data.profileCount} profile(s): ${result.data.path}`);
    } catch (error) {
      setBackupMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBackupBusy(false);
    }
  };

  return (
    <SettingSection title='Server'>
      <Show when={activeSession()} fallback={<p class='text-secondary-text leading-5'>No active server session.</p>}>
        {(session) => (
          <div class='flex flex-col gap-2'>
            <p>{session().username}</p>
            <p class='text-secondary-text leading-5'>{session().normalizedServerUrl}</p>
            <p class='text-secondary-text text-xs'>
              API {session().apiVersion}
              <Show when={session().serverType}>{(serverType) => <> · {serverType()}</>}</Show>
              <Show when={session().serverVersion}>{(serverVersion) => <> {serverVersion()}</>}</Show>
            </p>
          </div>
        )}
      </Show>
      <div class='flex flex-wrap gap-2'>
        <button type='button' disabled={backupBusy()} onClick={exportBackup}>
          Export Backup
        </button>
        <button type='button' disabled={backupBusy()} onClick={importBackup}>
          Import Backup
        </button>
      </div>
      <p class='text-secondary-text text-xs leading-5'>Backup includes server credentials. Keep the exported JSON private.</p>
      <Show when={backupMessage()}>{(message) => <p class='text-secondary-text text-xs leading-5'>{message()}</p>}</Show>
    </SettingSection>
  );
}

export default ServerSettings;
