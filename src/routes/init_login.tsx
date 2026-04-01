import { useNavigate } from '@solidjs/router';
import { createSignal, Show } from 'solid-js';
import { commands, type ConnectServerProfileRequest, type ConnectServerProfileResult } from '~/bindings';
import AuthForm, { AuthFormData } from '~/components/auth/AuthForm';
import ErrorMsg from '~/components/common/ErrorMsg';
import Header from '~/components/common/header/Header';
import { setSessionStore } from '~/stores/SessionStore';

function formatFailureMsg(result: ConnectServerProfileResult) {
  switch (result.status) {
    case 'auth_error':
      return `Auth Error: ${result.message}`;
    case 'network_error':
      return `Network Error: ${result.message}`;
    case 'server_error':
      return `Server Error: ${result.message}`;
    case 'unsupported_auth':
      return `Unsupported Error: ${result.message}`;
    case 'connected':
      return 'Connected';
  }
}

function InitLogin() {
  const navigate = useNavigate();

  const [submitting, setSubmitting] = createSignal<boolean>(false);
  const [submitError, setSubmitError] = createSignal<string | undefined>();

  async function submitConnection(formData: AuthFormData) {
    const { displayName, serverUrl, authKind, username, secret } = formData;
    setSubmitting(true);
    setSubmitError(undefined);

    try {
      const payload: ConnectServerProfileRequest = {
        profileId: null,
        displayName: null,
        serverUrl,
        auth:
          authKind === 'password'
            ? {
                kind: 'password',
                username: username,
                password: secret,
              }
            : {
                kind: 'api_key',
                apiKey: secret,
              },
      };

      const nextDisplayName = displayName.trim();
      if (nextDisplayName.length > 0) {
        payload.displayName = nextDisplayName;
      }

      const commandResult = await commands.connectServerProfile(payload);
      if (commandResult.status === 'error') {
        throw new Error(commandResult.error);
      }

      const result = commandResult.data;

      setSessionStore('profiles', result.profiles);
      switch (result.status) {
        case 'network_error':
        case 'server_error':
        case 'unsupported_auth':
        case 'auth_error':
          setSubmitError(formatFailureMsg(result));
          return;
        case 'connected':
          setSessionStore('activeSession', result.activeSession);
          navigate('/home');
          break;
      }
    } catch (invokeError) {
      console.error(invokeError);
      const errorMessage = invokeError instanceof Error ? invokeError.message : String(invokeError);
      setSubmitError(errorMessage);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div class='flex flex-col w-dvw h-dvh'>
      <Header title='Login to server' shouldShowProfiles={false} />
      <div class='flex flex-col w-full h-full items-center justify-center bg-zinc-200'>
        <div class='flex flex-col p-7 gap-3 bg-primary-plane border border-zinc-600 rounded-md'>
          <AuthForm onSubmit={(data) => submitConnection(data)} busy={submitting()} />

          <Show when={submitError()}>
            <ErrorMsg>{submitError()}</ErrorMsg>
          </Show>
        </div>
      </div>
    </div>
  );
}

export default InitLogin;
