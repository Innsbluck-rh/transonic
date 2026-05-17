import { Component, Show } from 'solid-js';
import { createStore } from 'solid-js/store';
import type { AuthKind } from '~/bindings';

export interface AuthFormData {
  displayName: string;
  serverUrl: string;
  authKind: AuthKind;
  username: string;
  secret: string;
}

interface AuthFormProps {
  defaultData?: AuthFormData;
  onSubmit: (data: AuthFormData) => void;
  busy: boolean;
}

const DEFAULT_SERVER_URL = 'https://demo.navidrome.org';
const DEFAULT_FORM_DATA: AuthFormData = {
  displayName: '',
  serverUrl: DEFAULT_SERVER_URL,
  authKind: 'password',
  username: '',
  secret: '',
} as const;

interface AuthFormFields {
  displayName: string;
  serverBaseUrl: string;
  serverPort: string;
  serverPath: string;
  authKind: AuthKind;
  username: string;
  secret: string;
}

function parseServerUrlFields(serverUrl: string): Pick<AuthFormFields, 'serverBaseUrl' | 'serverPort' | 'serverPath'> {
  try {
    const parsed = new URL(serverUrl);
    const port = parsed.port;
    parsed.port = '';
    parsed.pathname = '';
    parsed.search = '';
    parsed.hash = '';
    return {
      serverBaseUrl: parsed.toString().replace(/\/$/, ''),
      serverPort: port,
      serverPath: new URL(serverUrl).pathname === '/' ? '' : new URL(serverUrl).pathname.replace(/^\/+/, ''),
    };
  } catch {
    return {
      serverBaseUrl: '',
      serverPort: '',
      serverPath: '',
    };
  }
}

function toFormFields(data: AuthFormData): AuthFormFields {
  return {
    ...parseServerUrlFields(data.serverUrl),
    displayName: data.displayName,
    authKind: data.authKind,
    username: data.username,
    secret: data.secret,
  };
}

function buildServerUrl(fields: AuthFormFields) {
  const port = fields.serverPort.trim();
  const path = fields.serverPath.trim().replace(/^\/+/, '');
  const parsed = new URL(fields.serverBaseUrl.trim());
  if (port.length > 0) {
    parsed.port = port;
  }
  parsed.pathname = path.length > 0 ? `/${path}` : '';
  parsed.search = '';
  parsed.hash = '';
  return parsed.toString().replace(/\/$/, '');
}

function splitServerBaseUrl(rawUrl: string) {
  try {
    const parsed = new URL(rawUrl);
    const port = parsed.port;
    parsed.port = '';
    parsed.pathname = '';
    parsed.search = '';
    parsed.hash = '';
    return {
      serverBaseUrl: parsed.toString().replace(/\/$/, ''),
      serverPort: port,
    };
  } catch {
    return {
      serverBaseUrl: rawUrl,
      serverPort: '',
    };
  }
}

const AuthForm: Component<AuthFormProps> = (props) => {
  const [formData, setFormData] = createStore<AuthFormFields>(toFormFields(props.defaultData ?? DEFAULT_FORM_DATA));
  const resetForm = () => setFormData(toFormFields(DEFAULT_FORM_DATA));

  return (
    <form
      class='flex w-full flex-col gap-1'
      onSubmit={(e) => {
        e.preventDefault();
        props.onSubmit({
          displayName: formData.displayName,
          serverUrl: buildServerUrl(formData),
          authKind: formData.authKind,
          username: formData.username,
          secret: formData.secret,
        });
      }}
    >
      <label class='flex flex-col'>
        <span>Display name</span>
        <input
          type='text'
          class='bg-primary-surface border-primary-border min-w-0 rounded border px-3 py-2'
          value={formData.displayName}
          onInput={(event) => setFormData('displayName', event.currentTarget.value)}
          placeholder='Navidrome'
          autocomplete='off'
        />
      </label>

      <label class='flex flex-col'>
        <span>Server URL</span>
        <div class='grid grid-cols-[minmax(0,1fr)_7rem] gap-2'>
          <input
            type='text'
            class='bg-primary-surface border-primary-border min-w-0 rounded border px-3 py-2'
            value={formData.serverBaseUrl}
            onInput={(event) => setFormData('serverBaseUrl', event.currentTarget.value)}
            onBlur={(event) => {
              const split = splitServerBaseUrl(event.currentTarget.value);
              setFormData('serverBaseUrl', split.serverBaseUrl);
              if (split.serverPort.length > 0) {
                setFormData('serverPort', split.serverPort);
              }
            }}
            placeholder='https://your-server.example'
            autocomplete='url'
            required
          />
          <input
            class='bg-primary-surface border-primary-border min-w-0 rounded border px-3 py-2'
            type='number'
            min='1'
            max='65535'
            value={formData.serverPort}
            onInput={(event) => setFormData('serverPort', event.currentTarget.value)}
            placeholder='4533'
            autocomplete='off'
          />
        </div>
      </label>

      {/*<label class='contents items-center'>
        <span>Base path</span>
        <input
          type='text'
          class='bg-primary-surface border-primary-border min-w-0 rounded border px-3 py-2'
          value={formData.serverPath}
          onInput={(event) => setFormData('serverPath', event.currentTarget.value)}
          placeholder='rest'
          autocomplete='off'
        />
      </label>*/}

      <label class='contents items-center'>
        <span>Auth method</span>
        <select
          class='bg-primary-surface border-primary-border min-w-0 rounded border px-3 py-2'
          value={formData.authKind}
          onInput={(event) => setFormData('authKind', event.currentTarget.value as AuthKind)}
        >
          <option value='password'>Password token</option>
          <option value='api_key'>API key</option>
        </select>
      </label>

      <Show when={formData.authKind === 'password'}>
        <label class='contents items-center'>
          <span>Username</span>
          <input
            class='bg-primary-surface border-primary-border min-w-0 rounded border px-3 py-2'
            type='text'
            value={formData.username}
            onInput={(event) => setFormData('username', event.currentTarget.value)}
            placeholder='demo'
            autocomplete='username'
            required
          />
        </label>
      </Show>

      <label class='contents items-center'>
        <span>{formData.authKind === 'password' ? 'Password' : 'API key'}</span>
        <input
          class='bg-primary-surface border-primary-border min-w-0 rounded border px-3 py-2'
          type={formData.authKind === 'password' ? 'password' : 'text'}
          value={formData.secret}
          onInput={(event) => setFormData('secret', event.currentTarget.value)}
          placeholder={formData.authKind === 'password' ? 'Your password' : 'Paste an OpenSubsonic API key'}
          autocomplete={formData.authKind === 'password' ? 'current-password' : 'off'}
          required
        />
      </label>

      <div class='mt-3 flex flex-row gap-1'>
        <button class='disabled:text-zinc-400' type='submit' disabled={props.busy}>
          {props.busy ? 'Connecting...' : 'Connect'}
        </button>
        <button type='button' disabled={props.busy} onClick={resetForm}>
          Reset form
        </button>
      </div>
    </form>
  );
};

export default AuthForm;
