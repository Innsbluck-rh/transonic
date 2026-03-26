import { useNavigate } from '@solidjs/router';
import { invoke } from '@tauri-apps/api/core';
import AuthForm, { AuthFormData } from '~/components/auth/AuthForm';
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

export type ConnectServerProfileResult = ConnectedResult | ConnectionFailure;

function InitLogin() {
  const navigate = useNavigate();

  async function submitConnection(formData: AuthFormData) {
    const { displayName, serverUrl, authKind, username, secret } = formData;
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
      if (result.status === 'connected') {
        setSessionStore('activeSession', result.activeSession);
        navigate('/home');
      }
    } catch (invokeError) {
      console.error(invokeError);
    }
  }

  return (
    <div class='flex flex-col gap-1.5'>
      <Header title='Login to server' shouldShowProfiles={false} />

      <div class='flex flex-col p-3 m-3 gap-3  rounded-lg border border-zinc-700 '>
        <Heading2>Navidrome</Heading2>
        <AuthForm onSubmit={(data) => submitConnection(data)} busy={false} />
      </div>
    </div>
  );
}

export default InitLogin;
