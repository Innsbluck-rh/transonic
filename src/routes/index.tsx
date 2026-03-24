import { useNavigate } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";
import { onMount } from "solid-js";
import { loadBootstrapToStore } from "~/features/session/service";
import { AppBootstrap } from "~/models/bootstrap";

function Index() {
  const navigate = useNavigate();
  // load bootstrap and redirect
  onMount(async () => {
    const bootstrap = await invoke<AppBootstrap>("bootstrap_app_state");
    loadBootstrapToStore(bootstrap);
    switch (bootstrap.restoreStatus) {
      case "restored":
        navigate("/home");
        break;
      case "none":
      case "offline": // !! temporally assume logouted
      case "reauth_required": // !! temporally assume logouted
        navigate("/init_login");
        break;
    }
  });

  return <div>Loading...</div>;
}
export default Index;
