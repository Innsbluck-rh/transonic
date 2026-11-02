// Write global signals (like open/close) and functions of the dialogs which called from outside components.
// This will allow components to refresh its layout properly on hot reload (otherwise it doesn't.)

import { createSignal } from 'solid-js';

// AlbumArtOverlayDialog
const [albumArtOverlayDialogOpen, setAlbumArtOverlayDialogOpen] = createSignal(false);
const [albumArtOverlayCoverArtId, setAlbumArtOverlayCoverArtId] = createSignal<string | undefined>(undefined);

export function openAlbumArtOverlayDialog(albumCoverArtId: string) {
  setAlbumArtOverlayCoverArtId(albumCoverArtId);
  setAlbumArtOverlayDialogOpen(true);
}
export function closeAlbumArtOverlayDialog() {
  setAlbumArtOverlayCoverArtId(undefined);
  setAlbumArtOverlayDialogOpen(false);
}

export { albumArtOverlayCoverArtId, albumArtOverlayDialogOpen };

// PlaybackStreamInfoDialog
const [playbackStreamInfoDialogOpen, setPlaybackStreamInfoDialogOpen] = createSignal(false);

export function openPlaybackStreamInfoDialog() {
  setPlaybackStreamInfoDialogOpen(true);
}
export function closePlaybackStreamInfoDialog() {
  setPlaybackStreamInfoDialogOpen(false);
}

export { playbackStreamInfoDialogOpen };
