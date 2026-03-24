import { ActiveSession, SavedProfileSummary } from "./session";

export type RestoreStatus = "none" | "restored" | "offline" | "reauth_required";

export type AppBootstrap = {
  profiles: SavedProfileSummary[];
  activeSession: ActiveSession | null;
  restoreStatus: RestoreStatus;
  message?: string | null; // to be removed!!
};
