import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, createSignal, For, Show } from 'solid-js';
import type { PlaybackActualStreamInfo, PlaybackStreamInfo } from '~/bindings';
import { settingsStore } from '~/features/settings/service';
import { playbackStore } from '~/stores/PlaybackStore';
import Heading3 from '../Heading3';

const [open, setOpen] = createSignal(false);

export function openPlaybackStreamInfoDialog() {
  setOpen(true);
}
export function closePlaybackStreamInfoDialog() {
  setOpen(false);
}

interface InfoRow {
  key: string;
  value: string;
}

interface InfoSection {
  title: string;
  rows: InfoRow[];
}

function valueText(value: unknown): string {
  if (value === null || value === undefined || value === '') {
    return '(none)';
  }
  if (typeof value === 'boolean') {
    return value ? 'true' : 'false';
  }
  return String(value);
}

function pushRow(rows: InfoRow[], key: string, value: unknown) {
  rows.push({ key, value: valueText(value) });
}

function streamSection(title: string, info: PlaybackStreamInfo | null | undefined): InfoSection {
  const rows: InfoRow[] = [];

  if (!info) {
    pushRow(rows, 'status', null);
    return { title, rows };
  }

  pushRow(rows, 'requestKind', info.requestKind);
  pushRow(rows, 'songId', info.songId);
  pushRow(rows, 'songPath', info.songPath);
  pushRow(rows, 'sourceContentType', info.sourceContentType);
  pushRow(rows, 'sourceSuffix', info.sourceSuffix);
  pushRow(rows, 'sourceBitRate', info.sourceBitRate);
  pushRow(rows, 'sourceSize', info.sourceSize);
  pushRow(rows, 'streamMode', info.streamMode);
  pushRow(rows, 'rawStream', info.rawStream);
  pushRow(rows, 'streamFormat', info.streamFormat);
  pushRow(rows, 'streamMaxBitRate', info.streamMaxBitRate);
  pushRow(rows, 'streamOffsetSeconds', info.streamOffsetSeconds);
  pushRow(rows, 'requestedPositionMs', info.requestedPositionMs);
  pushRow(rows, 'localStartPositionMs', info.localStartPositionMs);
  pushRow(rows, 'autoplay', info.autoplay);
  pushRow(rows, 'streamEndpoint', info.streamEndpoint);
  pushRow(rows, 'streamUrl', info.streamUrl);

  return { title, rows };
}

function actualStreamSection(title: string, info: PlaybackActualStreamInfo | null | undefined): InfoSection {
  const rows: InfoRow[] = [];

  if (!info) {
    pushRow(rows, 'status', null);
    return { title, rows };
  }

  pushRow(rows, 'codec', info.codec);
  pushRow(rows, 'codecProfile', info.codecProfile);
  pushRow(rows, 'mimeType', info.mimeType);
  pushRow(rows, 'bitrate', info.bitrate);
  pushRow(rows, 'sampleRate', info.sampleRate);
  pushRow(rows, 'channels', info.channels);
  pushRow(rows, 'bitDepth', info.bitDepth);
  pushRow(rows, 'sampleFormat', info.sampleFormat);

  return { title, rows };
}

export const PlaybackStreamInfoDialog: Component = () => {
  const [copyState, setCopyState] = createSignal<string | null>(null);

  const status = () => playbackStore.status;
  const currentSong = createMemo(() => {
    const currentStatus = status();
    const currentIndex = currentStatus?.currentIndex;
    if (currentStatus === undefined || currentIndex === null || currentIndex === undefined) {
      return null;
    }
    return currentStatus.queue[currentIndex] ?? null;
  });

  const sections = createMemo<InfoSection[]>(() => {
    const currentStatus = status();
    const song = currentSong();
    const playbackRows: InfoRow[] = [];
    const gaplessRows: InfoRow[] = [];
    const settingsRows: InfoRow[] = [];
    const songRows: InfoRow[] = [];

    pushRow(playbackRows, 'playingState', currentStatus?.playingState);
    pushRow(playbackRows, 'interruptReason', currentStatus?.interruptReason);
    pushRow(playbackRows, 'pendingSeekPositionMs', currentStatus?.pendingSeekPositionMs);
    pushRow(playbackRows, 'currentPositionMs', currentStatus?.currentPositionMs);
    pushRow(playbackRows, 'currentIndex', currentStatus?.currentIndex);
    pushRow(playbackRows, 'currentSongId', currentStatus?.currentSongId);
    pushRow(playbackRows, 'error', currentStatus?.playbackError?.message);

    pushRow(gaplessRows, 'state', currentStatus?.gaplessStatus.state);
    pushRow(gaplessRows, 'message', currentStatus?.gaplessStatus.message);

    pushRow(settingsRows, 'streamMode', settingsStore.playback.streamMode);
    pushRow(settingsRows, 'transcodingBitrateLimit', settingsStore.playback.transcodingBitrateLimit);
    pushRow(settingsRows, 'useCustomTranscodingCodec', settingsStore.playback.useCustomTranscodingCodec);
    pushRow(settingsRows, 'transcodingCodec', settingsStore.playback.transcodingCodec);
    pushRow(settingsRows, 'gaplessPlaybackEnabled', settingsStore.playback.gaplessPlaybackEnabled);

    pushRow(songRows, 'title', song?.title);
    pushRow(songRows, 'artist', song?.artist);
    pushRow(songRows, 'album', song?.album);
    pushRow(songRows, 'path', song?.path);
    pushRow(songRows, 'contentType', song?.contentType);
    pushRow(songRows, 'suffix', song?.suffix);
    pushRow(songRows, 'bitRate', song?.bitRate);
    pushRow(songRows, 'size', song?.size);
    pushRow(songRows, 'duration', song?.duration);

    return [
      { title: 'playback', rows: playbackRows },
      { title: 'gapless', rows: gaplessRows },
      { title: 'settings', rows: settingsRows },
      { title: 'song', rows: songRows },
      actualStreamSection('actualStream', currentStatus?.actualStreamInfo),
      streamSection('activeStream', currentStatus?.activeStreamInfo),
      streamSection('preparedStream', currentStatus?.preparedStreamInfo),
      streamSection('lastStreamRequest', currentStatus?.lastStreamRequest),
    ];
  });

  const copyText = createMemo(() =>
    sections()
      .map((section) => `${section.title}\n${section.rows.map((row) => `${row.key}: ${row.value}`).join(', ')}`)
      .join('\n')
  );

  const close = () => {
    setCopyState(null);
    closePlaybackStreamInfoDialog();
  };

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(copyText());
      setCopyState('Copied!');
    } catch {
      setCopyState('Copy failed');
    }
  };

  return (
    <Show when={open()}>
      <div class='fixed inset-0 z-100 flex items-center justify-center bg-black/45 p-4' role='presentation'>
        <section
          class='border-primary-border bg-primary-plane text-primary-text flex max-h-[80dvh] w-full max-w-5xl flex-col overflow-hidden rounded border shadow-xl'
          role='dialog'
          aria-modal='true'
          aria-labelledby='playback-stream-info-title'
        >
          <header class='border-secondary-border flex items-center gap-3 border-b px-4 py-3'>
            <Icon class='text-accent text-xl' icon='material-symbols:info-outline' />
            <h2 id='playback-stream-info-title' class='text-primary-text min-w-0 flex-1 text-base'>
              Playback stream info
            </h2>
            <button class='button-borderless px-2' type='button' onClick={close} aria-label='Close playback stream info'>
              <Icon icon='material-symbols:close' />
            </button>
          </header>
          <div class='overflow-auto p-4'>
            <div class='flex flex-col gap-3'>
              <For each={sections()}>
                {(section) => (
                  <section class='min-w-0' aria-label={section.title}>
                    <Heading3 class='mb-1'>{section.title}</Heading3>
                    <p class='text-xs leading-5 break-all'>
                      <For each={section.rows}>
                        {(row, index) => (
                          <>
                            <span class='text-secondary-text shrink-0 text-xs'>{row.key}</span>
                            <span class='text-secondary-text shrink-0 text-xs'>:&nbsp;</span>
                            <span class='text-primary-text min-w-0 text-xs font-bold break-all'>{row.value}</span>
                            <Show when={index() < section.rows.length - 1}>
                              <span class='text-secondary-text'> | </span>
                            </Show>
                          </>
                        )}
                      </For>
                    </p>
                  </section>
                )}
              </For>
            </div>
          </div>
          <footer class='border-secondary-border flex items-center justify-end gap-2 border-t px-4 py-3'>
            <Show when={copyState()}>{(state) => <span class='text-secondary-text mr-auto text-xs'>{state()}</span>}</Show>
            <button class='flex items-center gap-1' type='button' onClick={copy}>
              <Icon icon='material-symbols:content-copy-outline' />
              <span>Copy</span>
            </button>
            <button type='button' onClick={close}>
              Close
            </button>
          </footer>
        </section>
      </div>
    </Show>
  );
};

export default PlaybackStreamInfoDialog;
