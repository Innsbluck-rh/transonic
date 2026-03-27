import { useNavigate } from '@solidjs/router';
import { invoke } from '@tauri-apps/api/core';
import { createSignal, Show } from 'solid-js';
import AuthForm, { AuthFormData } from '~/components/auth/AuthForm';
import ErrorMsg from '~/components/common/ErrorMsg';
import Heading2 from '~/components/common/Heading2';
import Header from '~/components/header/Header';
import { ActiveSession, SavedProfileSummary } from '~/models/session';
import { setSessionStore } from '~/stores/session/SessionStore';

type ConnectServerProfileRequest = {
  profileId?: string;
  displayName?: string;
  serverUrl: string;
  auth:
    | {
        kind: 'password';
        username: string;
        password: string;
      }
    | {
        kind: 'api_key';
        apiKey: string;
      };
};

type ConnectedResult = {
  status: 'connected';
  activeSession: ActiveSession;
  profiles: SavedProfileSummary[];
};

type ConnectionFailure =
  | {
      status: 'auth_error';
      message: string;
      code?: number | null;
      helpUrl?: string | null;
      activeSession?: ActiveSession | null;
      profiles: SavedProfileSummary[];
    }
  | {
      status: 'network_error';
      message: string;
      activeSession?: ActiveSession | null;
      profiles: SavedProfileSummary[];
    }
  | {
      status: 'server_error';
      message: string;
      code?: number | null;
      helpUrl?: string | null;
      activeSession?: ActiveSession | null;
      profiles: SavedProfileSummary[];
    }
  | {
      status: 'unsupported_auth';
      message: string;
      activeSession?: ActiveSession | null;
      profiles: SavedProfileSummary[];
    };

function formatFailureMsg(failure: ConnectionFailure) {
  let status = 'Unknown';
  switch (failure.status) {
    case 'auth_error':
      status = 'Auth Error';
      break;
    case 'network_error':
      status = 'Network Error';
      break;
    case 'server_error':
      status = 'Server Error';
      break;
    case 'unsupported_auth':
      status = 'Unsupported Error';
      break;
    default:
      break;
  }
  return `${status}: ${failure.message}`;
}

export type ConnectServerProfileResult = ConnectedResult | ConnectionFailure;

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
        serverUrl: serverUrl,
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

      const result = await invoke<ConnectServerProfileResult>('connect_server_profile', {
        payload,
      });

      setSessionStore('profiles', result.profiles);
      switch (result.status) {
        case 'network_error':
        case 'server_error':
        case 'unsupported_auth':
        case 'auth_error':
          throw new Error(formatFailureMsg(result as ConnectionFailure));
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
    <div class='flex flex-col gap-1.5'>
      <Header title='Login to server' shouldShowProfiles={false} />

      <div class='flex flex-col p-3 m-3 gap-3  rounded-lg border border-zinc-700 '>
        <Heading2>Navidrome</Heading2>
        <AuthForm onSubmit={(data) => submitConnection(data)} busy={submitting()} />

        <Show when={submitError()}>
          <ErrorMsg>{submitError()}</ErrorMsg>
        </Show>
      </div>
    </div>
  );
}

export default InitLogin;
