import { Component, createMemo, Show } from 'solid-js';
import type { PlaybackActualStreamInfo } from '~/bindings';
import { usePlaybackStatus } from '~/features/playback/usePlaybackStatus';

interface PlaybackStatusTextProps {
  class?: string;
}

function formatBitrate(value: number | null): string | null {
  if (!value || value <= 0) {
    return null;
  }
  return `${Math.round(value / 1000)}kbps`;
}

function formatSampleRate(value: number | null): string | null {
  if (!value || value <= 0) {
    return null;
  }
  const khz = value / 1000;
  return `${Number.isInteger(khz) ? khz.toFixed(0) : khz.toFixed(1)}kHz`;
}

function formatChannels(value: number | null): string | null {
  if (!value || value <= 0) {
    return null;
  }
  if (value === 1) {
    return 'mono';
  }
  if (value === 2) {
    return 'stereo';
  }
  return `${value}ch`;
}

function codecLabel(info: PlaybackActualStreamInfo): string | null {
  return info.codec ?? info.mimeType?.split('/').pop() ?? null;
}

function streamParts(info: PlaybackActualStreamInfo | null | undefined): string[] {
  if (!info) {
    return [];
  }
  return [
    formatBitrate(info.bitrate),
    formatSampleRate(info.sampleRate),
    formatChannels(info.channels),
    info.bitDepth ? `${info.bitDepth}bit` : null,
  ].filter((part): part is string => part !== null);
}

const PlaybackStatusText: Component<PlaybackStatusTextProps> = (props) => {
  const playbackStatus = usePlaybackStatus();
  const actualStreamInfo = createMemo(() => playbackStatus()?.actualStreamInfo ?? null);
  const parts = createMemo(() => streamParts(actualStreamInfo()));
  const codec = createMemo(() => {
    const info = actualStreamInfo();
    return info ? codecLabel(info) : null;
  });

  return (
    <Show when={actualStreamInfo()}>
      <p class={`text-secondary-text text-xs ${props.class}`}>
        <Show when={parts().length > 0}>
          {parts().join(' | ')}
          <Show when={codec()}>&nbsp;|&nbsp;</Show>
        </Show>
        <span class='text-secondary-text text-xs font-bold'>{codec() ?? 'unknown'}</span>
      </p>
    </Show>
  );
};

export default PlaybackStatusText;
