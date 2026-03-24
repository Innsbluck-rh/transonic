import { For, Show, createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { LastConnectionState, SavedProfileSummary } from "./models/session";
import { ActiveSession } from "./models/session";
import { sessionStore } from "./stores/session/SessionStore";

type AuthKind = "password" | "api_key";
type RestoreStatus = "none" | "restored" | "offline" | "reauth_required";

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
        title: "Load failed",
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
      sessionStore.profiles.find(
        (profile) => profile.profileId === selectedProfileId(),
      ) ?? null
    );
  }

  function applyBootstrap(bootstrap: AppBootstrap) {
    syncFormWithProfiles(
      bootstrap.profiles,
      bootstrap.activeSession?.profileId ?? null,
    );
    setFeedback(createBootstrapFeedback(bootstrap));
  }

  function applyConnectionResult(result: ConnectServerProfileResult) {
    // TBD: replace or add?
    replaceProfiles(result.profiles);
    if (result.status === "connected") {
      setActiveSession(result.activeSession);
      syncFormWithProfiles(result.profiles, result.activeSession.profileId);
      setSecret("");
      setFeedback({
        tone: "success",
        title: "Connected",
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
  }

  function startNewProfile() {
    resetForm();
    setFeedback({
      tone: "neutral",
      title: "New profile",
      message: "Enter a server URL and credentials.",
    });
  }

  // serverUrl+username+secretのinvoke呼び出しはどこかにutilizeしたいが、busyActionが邪魔
  // しかしながらbusyActionは全体の動作に関わるので、グローバルなsessionStoreに置いて上位DOMでオーバーレイを表示するなどすべきかも？
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
        title: "Connect failed",
        message: `The desktop command failed before the server handshake could finish: ${String(
          invokeError,
        )}`,
      });
    } finally {
      setBusyAction(null);
    }
  }

  // 同上。
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
        title: "Activation failed",
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

  // 同上。
  async function deleteProfile(profileId: string) {
    const profile = getProfileById(profileId);
    if (profile == null) {
      return;
    }
    const confirmed = window.confirm(
      `Delete profile "${profile.displayName}"?`,
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
      <Show
        when={!loading()}
        fallback={
          <section class="card status-card">
            <p>Loading...</p>
          </section>
        }
      >
        <section class="card profiles-card">
          <div class="card-header inline-header">
            <h2>Profiles</h2>
            <button
              type="button"
              class="secondary-button"
              onClick={startNewProfile}
            >
              New
            </button>
          </div>

          <Show
            when={sessionStore.profiles.length > 0}
            fallback={
              <article class="empty-state">
                <p>No profiles.</p>
              </article>
            }
          >
            <div class="profile-list">
              <For each={sessionStore.profiles}>
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
            when={sessionStore.activeSession}
            fallback={
              <article class="empty-state">
                <p>No active session.</p>
              </article>
            }
          >
            {(session) => (
              <section class="session-panel">
                <div class="card-header inline-header">
                  <h2>Session</h2>
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
              <h2>{selectedProfile() ? "Edit profile" : "New profile"}</h2>
            </div>
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
        title: "Restored",
        message: bootstrap.message,
      };
    case "offline":
      return {
        tone: "warning",
        title: "Offline",
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
        title: "Updated",
        message: bootstrap.message,
      };
  }
}

function createFailureFeedback(result: ConnectionFailure): Feedback {
  switch (result.status) {
    case "auth_error":
      return {
        tone: "warning",
        title: "Auth failed",
        message: result.message,
        code: result.code,
        helpUrl: result.helpUrl,
      };
    case "network_error":
      return {
        tone: "warning",
        title: "Network failed",
        message: result.message,
      };
    case "server_error":
      return {
        tone: "danger",
        title: "Server error",
        message: result.message,
        code: result.code,
        helpUrl: result.helpUrl,
      };
    case "unsupported_auth":
      return {
        tone: "warning",
        title: "Unsupported auth",
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
      return "Never";
    case "ok":
      return "Connected";
    case "offline":
      return "Offline";
    case "reauth_required":
      return "Re-auth";
  }
}

function formatBoolean(value: boolean) {
  return value ? "Yes" : "No";
}

export default App;
