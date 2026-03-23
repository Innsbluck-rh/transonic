import { For, Show, createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type AuthKind = "password" | "api_key";
type RestoreStatus = "none" | "restored" | "offline" | "reauth_required";
type LastConnectionState = "never" | "ok" | "offline" | "reauth_required";

type OpenSubsonicExtension = {
  name: string;
  versions: number[];
};

type CapabilityMatrix = {
  openSubsonic: boolean;
  rawExtensions: OpenSubsonicExtension[];
  apiKeyAuth: boolean;
  indexBasedQueue: boolean;
  playbackReport: boolean;
  transcoding: boolean;
  transcodeOffset: boolean;
  songLyrics: boolean;
};

type SavedProfileSummary = {
  profileId: string;
  displayName: string;
  normalizedServerUrl: string;
  authKind: AuthKind;
  username: string;
  lastConnectionState: LastConnectionState;
  isActive: boolean;
};

type ActiveSession = {
  profileId: string;
  normalizedServerUrl: string;
  username: string;
  authKind: AuthKind;
  apiVersion: string;
  serverType?: string | null;
  serverVersion?: string | null;
  capabilityMatrix: CapabilityMatrix;
};

type AppBootstrap = {
  profiles: SavedProfileSummary[];
  activeSession: ActiveSession | null;
  restoreStatus: RestoreStatus;
  message?: string | null;
};

type ConnectServerProfileRequest = {
  profileId?: string;
  displayName?: string;
  serverUrl: string;
  auth:
    | {
        kind: "password";
        username: string;
        password: string;
      }
    | {
        kind: "api_key";
        username: string;
        apiKey: string;
      };
};

type ConnectedResult = {
  status: "connected";
  activeSession: ActiveSession;
  profiles: SavedProfileSummary[];
};

type ConnectionFailure =
  | {
      status: "auth_error";
      message: string;
      code?: number | null;
      helpUrl?: string | null;
      activeSession?: ActiveSession | null;
      profiles: SavedProfileSummary[];
    }
  | {
      status: "network_error";
      message: string;
      activeSession?: ActiveSession | null;
      profiles: SavedProfileSummary[];
    }
  | {
      status: "server_error";
      message: string;
      code?: number | null;
      helpUrl?: string | null;
      activeSession?: ActiveSession | null;
      profiles: SavedProfileSummary[];
    }
  | {
      status: "unsupported_auth";
      message: string;
      activeSession?: ActiveSession | null;
      profiles: SavedProfileSummary[];
    };

type ConnectServerProfileResult = ConnectedResult | ConnectionFailure;

type FeedbackTone = "neutral" | "success" | "warning" | "danger";

type Feedback = {
  tone: FeedbackTone;
  title: string;
  message: string;
  code?: number | null;
  helpUrl?: string | null;
};

const DEFAULT_SERVER_URL = "https://demo.navidrome.org";

function App() {
  const [profiles, setProfiles] = createSignal<SavedProfileSummary[]>([]);
  const [activeSession, setActiveSession] = createSignal<ActiveSession | null>(
    null,
  );
  const [loading, setLoading] = createSignal(true);
  const [busyAction, setBusyAction] = createSignal<string | null>(null);
  const [selectedProfileId, setSelectedProfileId] = createSignal<string | null>(
    null,
  );
  const [displayName, setDisplayName] = createSignal("");
  const [serverUrl, setServerUrl] = createSignal(DEFAULT_SERVER_URL);
  const [authKind, setAuthKind] = createSignal<AuthKind>("password");
  const [username, setUsername] = createSignal("");
  const [secret, setSecret] = createSignal("");
  const [feedback, setFeedback] = createSignal<Feedback | null>(null);

  onMount(() => {
    void loadBootstrap();
  });

  async function loadBootstrap() {
    setLoading(true);

    try {
      const bootstrap = await invoke<AppBootstrap>("bootstrap_app_state");
      applyBootstrap(bootstrap);
    } catch (invokeError) {
      setFeedback({
        tone: "danger",
        title: "App bootstrap failed",
        message: `The desktop command failed before profiles could load: ${String(
          invokeError,
        )}`,
      });
    } finally {
      setLoading(false);
    }
  }

  function selectedProfile() {
    return (
      profiles().find((profile) => profile.profileId === selectedProfileId()) ??
      null
    );
  }

  function applyBootstrap(bootstrap: AppBootstrap) {
    setProfiles(bootstrap.profiles);
    setActiveSession(bootstrap.activeSession);
    syncFormWithProfiles(
      bootstrap.profiles,
      bootstrap.activeSession?.profileId ?? null,
    );
    setFeedback(createBootstrapFeedback(bootstrap));
  }

  function applyConnectionResult(result: ConnectServerProfileResult) {
    setProfiles(result.profiles);

    if (result.status === "connected") {
      setActiveSession(result.activeSession);
      syncFormWithProfiles(result.profiles, result.activeSession.profileId);
      setSecret("");
      setFeedback({
        tone: "success",
        title: "Server connected",
        message: `Saved and activated ${result.activeSession.username} on ${result.activeSession.normalizedServerUrl}.`,
      });
      return;
    }

    setActiveSession(result.activeSession ?? null);
    syncFormWithProfiles(
      result.profiles,
      result.activeSession?.profileId ?? null,
    );
    setFeedback(createFailureFeedback(result));
  }

  function syncFormWithProfiles(
    nextProfiles: SavedProfileSummary[],
    activeProfileId: string | null,
  ) {
    const preferredProfile =
      (activeProfileId != null
        ? nextProfiles.find((profile) => profile.profileId === activeProfileId)
        : undefined) ?? nextProfiles.find((profile) => profile.isActive);

    if (preferredProfile) {
      hydrateFormFromProfile(preferredProfile);
      return;
    }

    resetForm();
  }

  function hydrateFormFromProfile(profile: SavedProfileSummary) {
    setSelectedProfileId(profile.profileId);
    setDisplayName(profile.displayName);
    setServerUrl(profile.normalizedServerUrl);
    setAuthKind(profile.authKind);
    setUsername(profile.username);
    setSecret("");
  }

  function resetForm() {
    setSelectedProfileId(null);
    setDisplayName("");
    setServerUrl(DEFAULT_SERVER_URL);
    setAuthKind("password");
    setUsername("");
    setSecret("");
  }

  function startNewProfile() {
    resetForm();
    setFeedback({
      tone: "neutral",
      title: "Create a new profile",
      message:
        "Enter a server URL and credentials to save another Subsonic-compatible server.",
    });
  }

  async function submitConnection(event: SubmitEvent) {
    event.preventDefault();
    setBusyAction("connect");

    try {
      const payload: ConnectServerProfileRequest = {
        serverUrl: serverUrl(),
        auth:
          authKind() === "password"
            ? {
                kind: "password",
                username: username(),
                password: secret(),
              }
            : {
                kind: "api_key",
                username: username(),
                apiKey: secret(),
              },
      };

      const currentProfileId = selectedProfileId();
      if (currentProfileId) {
        payload.profileId = currentProfileId;
      }

      const nextDisplayName = displayName().trim();
      if (nextDisplayName.length > 0) {
        payload.displayName = nextDisplayName;
      }

      const result = await invoke<ConnectServerProfileResult>(
        "connect_server_profile",
        {
          payload,
        },
      );
      applyConnectionResult(result);
    } catch (invokeError) {
      setFeedback({
        tone: "danger",
        title: "Connection command failed",
        message: `The desktop command failed before the server handshake could finish: ${String(
          invokeError,
        )}`,
      });
    } finally {
      setBusyAction(null);
    }
  }

  async function activateProfile(profileId: string) {
    setBusyAction(`activate:${profileId}`);

    try {
      const result = await invoke<ConnectServerProfileResult>(
        "activate_server_profile",
        {
          payload: {
            profileId,
          },
        },
      );
      applyConnectionResult(result);
    } catch (invokeError) {
      setFeedback({
        tone: "danger",
        title: "Profile activation failed",
        message: `The desktop command failed before the saved profile could reconnect: ${String(
          invokeError,
        )}`,
      });
    } finally {
      setBusyAction(null);
    }
  }

  async function disconnectProfile() {
    setBusyAction("disconnect");

    try {
      const bootstrap = await invoke<AppBootstrap>("disconnect_active_profile");
      applyBootstrap(bootstrap);
    } catch (invokeError) {
      setFeedback({
        tone: "danger",
        title: "Disconnect failed",
        message: `The desktop command failed before the active profile could disconnect: ${String(
          invokeError,
        )}`,
      });
    } finally {
      setBusyAction(null);
    }
  }

  async function deleteProfile(profileId: string) {
    const profile = profiles().find((entry) => entry.profileId === profileId);
    if (profile == null) {
      return;
    }

    const confirmed = window.confirm(
      `Delete the saved profile "${profile.displayName}"?`,
    );
    if (!confirmed) {
      return;
    }

    setBusyAction(`delete:${profileId}`);

    try {
      const bootstrap = await invoke<AppBootstrap>("delete_server_profile", {
        payload: {
          profileId,
        },
      });
      applyBootstrap(bootstrap);
    } catch (invokeError) {
      setFeedback({
        tone: "danger",
        title: "Delete failed",
        message: `The desktop command failed before the profile could be deleted: ${String(
          invokeError,
        )}`,
      });
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <main class="app-shell">
      <section class="hero-panel">
        <p class="eyebrow">Phase 1 session foundation</p>
        <h1>Persist server profiles and recover the active session.</h1>
        <p class="hero-copy">
          The frontend stays thin. Rust owns saved profiles, capability
          discovery, credential storage, and automatic session restore.
        </p>
      </section>

      <Show
        when={!loading()}
        fallback={
          <section class="card status-card">
            <div class="card-header">
              <h2>Loading profiles</h2>
              <p>
                Reading the local profile store and attempting session restore.
              </p>
            </div>
          </section>
        }
      >
        <section class="card profiles-card">
          <div class="card-header inline-header">
            <div>
              <h2>Saved profiles</h2>
              <p>
                Multiple profiles are supported, but only one can be active at a
                time.
              </p>
            </div>
            <button
              type="button"
              class="secondary-button"
              onClick={startNewProfile}
            >
              New profile
            </button>
          </div>

          <Show
            when={profiles().length > 0}
            fallback={
              <article class="empty-state">
                <h3>No saved profiles yet.</h3>
                <p>Create a server profile to begin the Phase 1 flow.</p>
              </article>
            }
          >
            <div class="profile-list">
              <For each={profiles()}>
                {(profile) => (
                  <article
                    classList={{
                      "profile-item": true,
                      active: profile.isActive,
                      selected: selectedProfileId() === profile.profileId,
                    }}
                    onClick={() => hydrateFormFromProfile(profile)}
                  >
                    <div class="profile-item-top">
                      <h3>{profile.displayName}</h3>
                      <span
                        class={`status-pill ${profile.lastConnectionState}`}
                      >
                        {formatConnectionState(profile.lastConnectionState)}
                      </span>
                    </div>

                    <p class="profile-url">{profile.normalizedServerUrl}</p>
                    <p class="profile-meta">
                      {profile.username} | {formatAuthKind(profile.authKind)}
                    </p>

                    <div class="profile-actions">
                      <button
                        type="button"
                        class="ghost-button"
                        onClick={(event) => {
                          event.stopPropagation();
                          hydrateFormFromProfile(profile);
                        }}
                      >
                        Edit
                      </button>
                      <button
                        type="button"
                        class="ghost-button"
                        disabled={busyAction() !== null}
                        onClick={(event) => {
                          event.stopPropagation();
                          void activateProfile(profile.profileId);
                        }}
                      >
                        {busyAction() === `activate:${profile.profileId}`
                          ? "Connecting..."
                          : "Activate"}
                      </button>
                      <button
                        type="button"
                        class="ghost-button danger"
                        disabled={busyAction() !== null}
                        onClick={(event) => {
                          event.stopPropagation();
                          void deleteProfile(profile.profileId);
                        }}
                      >
                        {busyAction() === `delete:${profile.profileId}`
                          ? "Deleting..."
                          : "Delete"}
                      </button>
                    </div>
                  </article>
                )}
              </For>
            </div>
          </Show>
        </section>

        <section class="card workspace-card">
          <Show when={feedback()}>
            {(currentFeedback) => (
              <article class={`feedback-card ${currentFeedback().tone}`}>
                <h3>{currentFeedback().title}</h3>
                <p>{currentFeedback().message}</p>
                <Show when={currentFeedback().code != null}>
                  <p class="feedback-meta">
                    Subsonic error code: {currentFeedback().code}
                  </p>
                </Show>
                <Show when={currentFeedback().helpUrl}>
                  <a
                    class="result-link"
                    href={currentFeedback().helpUrl ?? ""}
                    target="_blank"
                    rel="noreferrer"
                  >
                    Open server help
                  </a>
                </Show>
              </article>
            )}
          </Show>

          <Show
            when={activeSession()}
            fallback={
              <article class="empty-state">
                <h3>No active session.</h3>
                <p>
                  Activate a saved profile or submit the form to establish one.
                </p>
              </article>
            }
          >
            {(session) => (
              <section class="session-panel">
                <div class="card-header inline-header">
                  <div>
                    <h2>Active session</h2>
                    <p>
                      The Rust backend is the source of truth for this session
                      snapshot.
                    </p>
                  </div>
                  <button
                    type="button"
                    class="secondary-button"
                    disabled={busyAction() !== null}
                    onClick={() => void disconnectProfile()}
                  >
                    {busyAction() === "disconnect"
                      ? "Disconnecting..."
                      : "Disconnect"}
                  </button>
                </div>

                <dl class="summary-grid">
                  <div>
                    <dt>User</dt>
                    <dd>{session().username}</dd>
                  </div>
                  <div>
                    <dt>Auth</dt>
                    <dd>{formatAuthKind(session().authKind)}</dd>
                  </div>
                  <div>
                    <dt>API base</dt>
                    <dd>{session().normalizedServerUrl}</dd>
                  </div>
                  <div>
                    <dt>API version</dt>
                    <dd>{session().apiVersion}</dd>
                  </div>
                  <div>
                    <dt>Server type</dt>
                    <dd>{session().serverType ?? "Not reported"}</dd>
                  </div>
                  <div>
                    <dt>Server version</dt>
                    <dd>{session().serverVersion ?? "Not reported"}</dd>
                  </div>
                  <div>
                    <dt>OpenSubsonic</dt>
                    <dd>
                      {session().capabilityMatrix.openSubsonic ? "Yes" : "No"}
                    </dd>
                  </div>
                </dl>

                <div class="extensions-panel">
                  <div class="extension-flags">
                    <span>
                      API key auth:{" "}
                      {formatBoolean(session().capabilityMatrix.apiKeyAuth)}
                    </span>
                    <span>
                      Index queue:{" "}
                      {formatBoolean(
                        session().capabilityMatrix.indexBasedQueue,
                      )}
                    </span>
                    <span>
                      Playback report:{" "}
                      {formatBoolean(session().capabilityMatrix.playbackReport)}
                    </span>
                    <span>
                      Transcoding:{" "}
                      {formatBoolean(session().capabilityMatrix.transcoding)}
                    </span>
                    <span>
                      Offset support:{" "}
                      {formatBoolean(
                        session().capabilityMatrix.transcodeOffset,
                      )}
                    </span>
                    <span>
                      Lyrics:{" "}
                      {formatBoolean(session().capabilityMatrix.songLyrics)}
                    </span>
                  </div>

                  <Show
                    when={session().capabilityMatrix.rawExtensions.length > 0}
                    fallback={
                      <p class="extensions-empty">
                        No OpenSubsonic extensions were reported.
                      </p>
                    }
                  >
                    <div class="extension-list">
                      <For each={session().capabilityMatrix.rawExtensions}>
                        {(extension) => (
                          <span class="extension-chip">
                            {extension.name} ({extension.versions.join(", ")})
                          </span>
                        )}
                      </For>
                    </div>
                  </Show>
                </div>
              </section>
            )}
          </Show>

          <section class="form-panel">
            <div class="card-header">
              <h2>
                {selectedProfile()
                  ? "Update saved profile"
                  : "Create saved profile"}
              </h2>
              <p>
                {selectedProfile()
                  ? "Submitting replaces the selected profile's connection settings after a successful handshake."
                  : "Submitting creates a new saved profile and activates it immediately."}
              </p>
            </div>

            <form class="connection-form" onSubmit={submitConnection}>
              <label class="field">
                <span>Display name</span>
                <input
                  type="text"
                  value={displayName()}
                  onInput={(event) => setDisplayName(event.currentTarget.value)}
                  placeholder="Office Navidrome"
                  autocomplete="off"
                />
              </label>

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
                <span>Auth method</span>
                <select
                  value={authKind()}
                  onInput={(event) =>
                    setAuthKind(event.currentTarget.value as AuthKind)
                  }
                >
                  <option value="password">Password token</option>
                  <option value="api_key">API key</option>
                </select>
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
                <span>
                  {authKind() === "password" ? "Password" : "API key"}
                </span>
                <input
                  type={authKind() === "password" ? "password" : "text"}
                  value={secret()}
                  onInput={(event) => setSecret(event.currentTarget.value)}
                  placeholder={
                    authKind() === "password"
                      ? "Your account password"
                      : "Paste an OpenSubsonic API key"
                  }
                  autocomplete={
                    authKind() === "password" ? "current-password" : "off"
                  }
                  required
                />
              </label>

              <div class="form-actions">
                <button
                  type="submit"
                  class="primary-button"
                  disabled={busyAction() !== null}
                >
                  {busyAction() === "connect"
                    ? "Connecting..."
                    : selectedProfile()
                      ? "Save and reconnect"
                      : "Create and connect"}
                </button>
                <button
                  type="button"
                  class="secondary-button"
                  disabled={busyAction() !== null}
                  onClick={startNewProfile}
                >
                  Reset form
                </button>
              </div>
            </form>
          </section>
        </section>
      </Show>
    </main>
  );
}

function createBootstrapFeedback(bootstrap: AppBootstrap): Feedback | null {
  if (bootstrap.message == null || bootstrap.message.length === 0) {
    return null;
  }

  switch (bootstrap.restoreStatus) {
    case "restored":
      return {
        tone: "success",
        title: "Session restored",
        message: bootstrap.message,
      };
    case "offline":
      return {
        tone: "warning",
        title: "Last active server is offline",
        message: bootstrap.message,
      };
    case "reauth_required":
      return {
        tone: "warning",
        title: "Credentials required",
        message: bootstrap.message,
      };
    case "none":
      return {
        tone: "neutral",
        title: "Profile state updated",
        message: bootstrap.message,
      };
  }
}

function createFailureFeedback(result: ConnectionFailure): Feedback {
  switch (result.status) {
    case "auth_error":
      return {
        tone: "warning",
        title: "Authentication failed",
        message: result.message,
        code: result.code,
        helpUrl: result.helpUrl,
      };
    case "network_error":
      return {
        tone: "warning",
        title: "Network request failed",
        message: result.message,
      };
    case "server_error":
      return {
        tone: "danger",
        title: "Server returned an unexpected response",
        message: result.message,
        code: result.code,
        helpUrl: result.helpUrl,
      };
    case "unsupported_auth":
      return {
        tone: "warning",
        title: "Server does not support this auth mode",
        message: result.message,
      };
  }
}

function formatAuthKind(authKind: AuthKind) {
  switch (authKind) {
    case "password":
      return "Password token";
    case "api_key":
      return "API key";
  }
}

function formatConnectionState(connectionState: LastConnectionState) {
  switch (connectionState) {
    case "never":
      return "Never connected";
    case "ok":
      return "Healthy";
    case "offline":
      return "Offline";
    case "reauth_required":
      return "Re-auth required";
  }
}

function formatBoolean(value: boolean) {
  return value ? "Yes" : "No";
}

export default App;
