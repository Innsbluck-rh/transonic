import { Show, createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type ConnectionSuccess = {
  status: "success";
  normalizedServerUrl: string;
  username: string;
  apiVersion: string;
  serverType?: string | null;
  serverVersion?: string | null;
  openSubsonic?: boolean | null;
};

type ConnectionFailure =
  | {
      status: "auth_error";
      message: string;
      code?: number | null;
      helpUrl?: string | null;
    }
  | {
      status: "network_error";
      message: string;
    }
  | {
      status: "server_error";
      message: string;
      code?: number | null;
      helpUrl?: string | null;
    };

type ConnectionResult = ConnectionSuccess | ConnectionFailure;

function App() {
  const [serverUrl, setServerUrl] = createSignal("https://demo.navidrome.org");
  const [username, setUsername] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [submitting, setSubmitting] = createSignal(false);
  const [error, setError] = createSignal<ConnectionFailure | null>(null);
  const [session, setSession] = createSignal<ConnectionSuccess | null>(null);

  async function submitConnection(event: SubmitEvent) {
    event.preventDefault();
    setSubmitting(true);
    setError(null);

    try {
      const result = await invoke<ConnectionResult>("check_subsonic_connection", {
        payload: {
          serverUrl: serverUrl(),
          username: username(),
          password: password(),
        },
      });

      // The backend already classifies the failure bucket, so the UI only branches on status.
      if (result.status === "success") {
        setSession(result);
        // The password is only needed for the one-shot connectivity check.
        setPassword("");
        return;
      }

      setError(result);
    } catch (invokeError) {
      setError({
        status: "network_error",
        message: `The desktop command failed before the request could finish: ${String(
          invokeError,
        )}`,
      });
    } finally {
      setSubmitting(false);
    }
  }

  function resetSession() {
    setSession(null);
    setError(null);
  }

  return (
    <main class="app-shell">
      <section class="hero-panel">
        <p class="eyebrow">Rust-backed connectivity check</p>
        <h1>Connect one Subsonic-compatible server.</h1>
        <p class="hero-copy">
          This screen only proves that Tauri can invoke Rust, build Subsonic
          token authentication, and classify the result of a single API call.
        </p>
      </section>

      <Show
        when={session()}
        fallback={
          <section class="card">
            <div class="card-header">
              <h2>Server login</h2>
              <p>Enter the server base URL and credentials for a one-shot ping.</p>
            </div>

            <form class="connection-form" onSubmit={submitConnection}>
              <label class="field">
                <span>Server URL</span>
                <input
                  type="url"
                  value={serverUrl()}
                  onInput={(event) => setServerUrl(event.currentTarget.value)}
                  placeholder="https://your-server.example"
                  autocomplete="url"
                  required
                />
              </label>

              <label class="field">
                <span>Username</span>
                <input
                  type="text"
                  value={username()}
                  onInput={(event) => setUsername(event.currentTarget.value)}
                  placeholder="demo"
                  autocomplete="username"
                  required
                />
              </label>

              <label class="field">
                <span>Password</span>
                <input
                  type="password"
                  value={password()}
                  onInput={(event) => setPassword(event.currentTarget.value)}
                  placeholder="Your account password"
                  autocomplete="current-password"
                  required
                />
              </label>

              <button type="submit" class="primary-button" disabled={submitting()}>
                {submitting() ? "Checking connection..." : "Connect"}
              </button>
            </form>

            <Show when={error()}>
              {(result) => (
                <article class={`result-card ${result().status}`}>
                  <h3>{formatResultTitle(result())}</h3>
                  <p>{result().message}</p>
                  <Show when={getResultCode(result()) !== null}>
                    <p class="result-meta">
                      Subsonic error code: {getResultCode(result())}
                    </p>
                  </Show>
                  <Show when={getResultHelpUrl(result()) !== null}>
                    <a
                      class="result-link"
                      href={getResultHelpUrl(result()) ?? ""}
                      target="_blank"
                      rel="noreferrer"
                    >
                      Open server help
                    </a>
                  </Show>
                </article>
              )}
            </Show>
          </section>
        }
      >
        {(connected) => (
          <section class="card home-card">
            <div class="card-header">
              <h2>Dummy home</h2>
              <p>
                The real player does not exist yet. This screen only confirms a
                successful authenticated handshake.
              </p>
            </div>

            <dl class="summary-grid">
              <div>
                <dt>User</dt>
                <dd>{connected().username}</dd>
              </div>
              <div>
                <dt>API base</dt>
                <dd>{connected().normalizedServerUrl}</dd>
              </div>
              <div>
                <dt>API version</dt>
                <dd>{connected().apiVersion}</dd>
              </div>
              <div>
                <dt>Server type</dt>
                <dd>{connected().serverType ?? "Not reported"}</dd>
              </div>
              <div>
                <dt>Server version</dt>
                <dd>{connected().serverVersion ?? "Not reported"}</dd>
              </div>
              <div>
                <dt>OpenSubsonic</dt>
                <dd>
                  {connected().openSubsonic == null
                    ? "Not reported"
                    : connected().openSubsonic
                      ? "Yes"
                      : "No"}
                </dd>
              </div>
            </dl>

            <button type="button" class="secondary-button" onClick={resetSession}>
              Use another server
            </button>
          </section>
        )}
      </Show>
    </main>
  );
}

function formatResultTitle(result: ConnectionFailure) {
  switch (result.status) {
    case "auth_error":
      return "Authentication failed";
    case "network_error":
      return "Network request failed";
    case "server_error":
      return "Server returned an unexpected response";
  }

  return "Connection failed";
}

function getResultCode(result: ConnectionFailure) {
  return "code" in result && result.code != null ? result.code : null;
}

function getResultHelpUrl(result: ConnectionFailure) {
  return "helpUrl" in result && result.helpUrl != null ? result.helpUrl : null;
}

export default App;
