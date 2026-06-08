import { Icon } from '@iconify-icon/solid';
import { createMemo, For, Show } from 'solid-js';
import type { PlaybackTranscodingCodec } from '~/bindings';
import { setPlaybackSetting, settingsStore } from '~/features/settings/service';
import { sessionStore } from '~/stores/SessionStore';
import Heading3 from '../common/Heading3';
import SettingSection from './SettingSection';

const TRANSCODING_CODEC_LABELS: Record<PlaybackTranscodingCodec, string> = {
  auto: 'Auto',
  mp3: 'MP3',
  flac: 'FLAC',
  aac: 'AAC',
  alac: 'ALAC',
  vorbis: 'Vorbis',
  opus: 'Opus',
};

function parseStreamBitrateLimit(value: string) {
  const bitrate = Number.parseInt(value, 10);
  return Number.isFinite(bitrate) && bitrate > 0 ? bitrate : 320;
}

function PlaybackSettings() {
  const transcodingCodecOptions = createMemo<PlaybackTranscodingCodec[]>(() => {
    const options = new Set<PlaybackTranscodingCodec>(['auto', 'mp3']);
    for (const codec of sessionStore.playbackCapabilities.transcodingCodecs ?? []) {
      options.add(codec);
    }
    options.add(settingsStore.playback.transcodingCodec);
    return [...options];
  });
  const selectedTranscodingCodec = (): PlaybackTranscodingCodec =>
    settingsStore.playback.useCustomTranscodingCodec ? settingsStore.playback.transcodingCodec : 'mp3';

  return (
    <SettingSection title='Playback'>
      <form class='flex flex-col gap-1'>
        <Heading3 class='mb-3'>Gapless Playback</Heading3>
        <Show when={sessionStore.playbackCapabilities.gaplessPlayback}>
          <label class='flex items-center justify-between gap-4'>
            <span>Gapless Playback</span>
            <input
              type='checkbox'
              checked={settingsStore.playback.gaplessPlaybackEnabled}
              onChange={(event) => setPlaybackSetting('gaplessPlaybackEnabled', event.currentTarget.checked)}
            />
          </label>
        </Show>
      </form>
      <form class='mt-2 flex flex-col gap-1'>
        <Heading3 class='mb-3'>Stream Mode</Heading3>
        <label for='raw_stream' class='hover:bg-primary-hover has-checked:bg-primary-hover flex items-center gap-3 rounded-md p-2'>
          <input
            name='stream_mode'
            type='radio'
            id='raw_stream'
            checked={settingsStore.playback.streamMode === 'raw'}
            onChange={() => setPlaybackSetting('streamMode', 'raw')}
          />
          <Icon icon='material-symbols:audio-file-rounded' class='text-2xl' />
          <div class='flex flex-col'>
            <p class='font-bold'>Raw Stream</p>
            <p class='text-secondary-text'>Stream raw audio files.</p>
          </div>
        </label>
        <label for='transcoding_stream' class='hover:bg-primary-hover has-checked:bg-primary-hover flex items-center gap-3 rounded-md p-2'>
          <input
            name='stream_mode'
            type='radio'
            id='transcoding_stream'
            checked={settingsStore.playback.streamMode === 'transcoding'}
            onChange={() => setPlaybackSetting('streamMode', 'transcoding')}
          />
          <Icon icon='material-symbols:replace-audio-rounded' class='text-2xl' />
          <div class='flex flex-col'>
            <p class='font-bold'>Transcoding Stream</p>
            <p class='text-secondary-text'>Stream audio files via transcoding.</p>
          </div>
        </label>
      </form>

      <Show when={settingsStore.playback.streamMode === 'transcoding'}>
        <form class='mt-2 flex flex-col gap-3'>
          <label class='mx-2 flex items-center justify-between gap-4'>
            <span>Stream Bitrate Limit</span>
            <select
              class='rounded border border-[var(--border-subtle)] bg-[var(--bg-secondary)] px-2 py-1 text-sm text-[var(--text-primary)]'
              value={settingsStore.playback.transcodingBitrateLimit}
              onChange={(event) => setPlaybackSetting('transcodingBitrateLimit', parseStreamBitrateLimit(event.currentTarget.value))}
            >
              <option value={96}>96 kbps</option>
              <option value={128}>128 kbps</option>
              <option value={192}>192 kbps</option>
              <option value={320}>320 kbps</option>
            </select>
          </label>
          <label class='mx-2 flex items-center justify-between gap-4'>
            <span>Use Custom Codec (unstable)</span>
            <input
              type='checkbox'
              checked={settingsStore.playback.useCustomTranscodingCodec}
              onChange={(event) => setPlaybackSetting('useCustomTranscodingCodec', event.currentTarget.checked)}
            />
          </label>
          <label class='mx-2 flex items-center justify-between gap-4'>
            <span>Transcoding Codec</span>
            <select
              class='rounded border border-[var(--border-subtle)] bg-[var(--bg-secondary)] px-2 py-1 text-sm text-[var(--text-primary)] disabled:opacity-70'
              disabled={!settingsStore.playback.useCustomTranscodingCodec}
              value={selectedTranscodingCodec()}
              onChange={(event) => setPlaybackSetting('transcodingCodec', event.currentTarget.value as PlaybackTranscodingCodec)}
            >
              <For each={transcodingCodecOptions()}>{(codec) => <option value={codec}>{TRANSCODING_CODEC_LABELS[codec]}</option>}</For>
            </select>
          </label>
        </form>
      </Show>
    </SettingSection>
  );
}

export default PlaybackSettings;
