import { Component, Show } from "solid-js";
import { createStore } from "solid-js/store";
import { AuthKind } from "~/models/session";

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

const DEFAULT_SERVER_URL = "https://demo.navidrome.org";
const DEFAULT_FORM_DATA: AuthFormData = {
  displayName: "",
  serverUrl: DEFAULT_SERVER_URL,
  authKind: "password",
  username: "",
  secret: "",
} as const;

const AuthForm: Component<AuthFormProps> = (props) => {
  const [formData, setFormData] = createStore<AuthFormData>(
    props.defaultData ?? DEFAULT_FORM_DATA,
  );

  return (
    <form
      class="connection-form"
      onSubmit={(e) => {
        e.preventDefault();
        props.onSubmit(formData);
      }}
    >
      <label class="field">
        <span>Display name</span>
        <input
          type="text"
          value={formData.displayName}
          onInput={(event) =>
            setFormData("displayName", event.currentTarget.value)
          }
          placeholder="Office Navidrome"
          autocomplete="off"
        />
      </label>

      <label class="field">
        <span>Server URL</span>
        <input
          type="url"
          value={formData.serverUrl}
          onInput={(event) =>
            setFormData("serverUrl", event.currentTarget.value)
          }
          placeholder="https://your-server.example"
          autocomplete="url"
          required
        />
      </label>

      <label class="field">
        <span>Auth method</span>
        <select
          value={formData.authKind}
          onInput={(event) =>
            setFormData("authKind", event.currentTarget.value as AuthKind)
          }
        >
          <option value="password">Password token</option>
          <option value="api_key">API key</option>
        </select>
      </label>

      <Show when={formData.authKind === "password"}>
        <label class="field">
          <span>Username</span>
          <input
            type="text"
            value={formData.username}
            onInput={(event) =>
              setFormData("username", event.currentTarget.value)
            }
            placeholder="demo"
            autocomplete="username"
            required
          />
        </label>
      </Show>

      <label class="field">
        <span>{formData.authKind === "password" ? "Password" : "API key"}</span>
        <input
          type={formData.authKind === "password" ? "password" : "text"}
          value={formData.secret}
          onInput={(event) => setFormData("secret", event.currentTarget.value)}
          placeholder={
            formData.authKind === "password"
              ? "Your account password"
              : "Paste an OpenSubsonic API key"
          }
          autocomplete={
            formData.authKind === "password" ? "current-password" : "off"
          }
          required
        />
      </label>

      <div class="form-actions">
        <button type="submit" class="primary-button" disabled={props.busy}>
          {props.busy ? "Connecting..." : "Connect"}
        </button>
        <button
          type="button"
          class="secondary-button"
          disabled={props.busy}
          onClick={() => setFormData(DEFAULT_FORM_DATA)}
        >
          Reset form
        </button>
      </div>
    </form>
  );
};

export default AuthForm;
