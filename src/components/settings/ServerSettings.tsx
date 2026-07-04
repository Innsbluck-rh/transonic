import { Icon } from '@iconify-icon/solid';
import { useNavigate } from '@solidjs/router';
import { createSignal, Show } from 'solid-js';
import { commands } from '~/bindings';
import { loadBootstrapToStore } from '~/features/session/service';
import { sessionStore } from '~/stores/SessionStore';
import Heading3 from '../common/text/Heading3';
import SettingSection from './SettingSection';

function ServerSettings() {
  const navigate = useNavigate();
  const activeSession = () => sessionStore.activeSession;
  const [backupMessage, setBackupMessage] = createSignal<string | null>(null);
  const [backupBusy, setBackupBusy] = createSignal(false);
  const [logoutConfirm, setLogoutConfirm] = createSignal(false);
  const [logoutBusy, setLogoutBusy] = createSignal(false);
  const [logoutError, setLogoutError] = createSignal<string | null>(null);

  const logout = async () => {
    const session = activeSession();
    if (!session) {
      return;
    }
    setLogoutBusy(true);
    setLogoutError(null);
    try {
      const result = await commands.deleteServerProfile({ profileId: session.profileId });
      if (result.status === 'error') {
        setLogoutError(result.error);
        return;
      }
      await loadBootstrapToStore(result.data);
      setLogoutConfirm(false);
      navigate('/init_login', { replace: true });
    } catch (error) {
      setLogoutError(error instanceof Error ? error.message : String(error));
    } finally {
      setLogoutBusy(false);
    }
  };

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
      <Heading3>Server</Heading3>
      <Show when={activeSession()} fallback={<p class='text-secondary-text leading-5'>No active server session.</p>}>
        {(session) => (
          <div class='flex flex-col gap-3'>
            <div class='flex gap-3'>
              <Icon icon='material-symbols:dns' class='text-xl' />
              <div class='flex flex-col'>
                <p class='text-sm font-bold'>{session().normalizedServerUrl}</p>
                <p class='text-secondary-text text-xs'>
                  API {session().apiVersion}
                  <Show when={session().serverType}>{(serverType) => <> · {serverType()}</>}</Show>
                  <Show when={session().serverVersion}>{(serverVersion) => <> {serverVersion()}</>}</Show>
                </p>
              </div>
            </div>
            <div class='flex items-center gap-3'>
              <Icon icon='material-symbols:account-circle' class='text-xl' />
              <p>{session().username}</p>
            </div>
            <button type='button' class='flex w-fit items-center gap-1 text-red-500' onClick={() => setLogoutConfirm(true)}>
              <Icon icon='material-symbols:logout' />
              <span>Logout</span>
            </button>
          </div>
        )}
      </Show>

      <Heading3 class='mt-3'>Backup</Heading3>
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

      <Show when={logoutConfirm()}>
        <div class='fixed inset-0 z-100 flex items-center justify-center bg-black/45 p-4' role='presentation'>
          <section
            class='border-primary-border bg-primary-plane text-primary-text flex w-full max-w-md flex-col gap-4 rounded border p-4 shadow-xl'
            role='alertdialog'
            aria-modal='true'
            aria-labelledby='logout-title'
          >
            <h2 id='logout-title' class='flex items-center gap-2 text-base'>
              <Icon icon='material-symbols:logout' class='text-red-500' />
              Logout
            </h2>
            <p class='text-secondary-text text-sm leading-5'>
              This removes the saved credentials and cached data for this server from this device. You will need to sign in again.
            </p>
            <Show when={logoutError()}>{(error) => <p class='text-xs leading-5 text-red-500'>{error()}</p>}</Show>
            <div class='flex justify-end gap-2'>
              <button type='button' disabled={logoutBusy()} onClick={() => setLogoutConfirm(false)}>
                Cancel
              </button>
              <button type='button' class='text-red-500' disabled={logoutBusy()} onClick={logout}>
                {logoutBusy() ? 'Logging out…' : 'Logout'}
              </button>
            </div>
          </section>
        </div>
      </Show>
    </SettingSection>
  );
}

export default ServerSettings;
