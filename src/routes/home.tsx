import { useNavigate } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";
import { onMount, Show } from "solid-js";
import {
  getProfileById,
  loadBootstrapToStore,
} from "~/features/session/service";
import { AppBootstrap } from "~/models/bootstrap";
import { sessionStore } from "~/stores/session/SessionStore";

function Home() {
  const navigate = useNavigate();
  onMount(async () => {
    const bootstrap = await invoke<AppBootstrap>("bootstrap_app_state");
    loadBootstrapToStore(bootstrap);

    switch (bootstrap.restoreStatus) {
      case "restored":
        // OK
        break;
      case "none":
        if (!bootstrap.activeSession) navigate("/init_login"); // FIXME: terrible workaround maybe
        break;
      case "offline": // !! temporally assume logouted
      case "reauth_required": // !! temporally assume logouted
        navigate("/init_login");
        break;
    }
  });

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
    try {
      const bootstrap = await invoke<AppBootstrap>("delete_server_profile", {
        payload: {
          profileId,
        },
      });
      loadBootstrapToStore(bootstrap);
    } catch (invokeError) {
      console.error(invokeError);
    }
  }

  return (
    <Show
      when={sessionStore.activeSession}
      fallback={
        <div>
          <p>seems like there's no active profile.</p>
        </div>
      }
    >
      <div>
        <h2>Welcome, {sessionStore.activeSession!.username}!</h2>
        <button
          onClick={() => deleteProfile(sessionStore.activeSession!.profileId)}
        >
          logout
        </button>
      </div>
    </Show>
  );
}

export default Home;
