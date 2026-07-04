import { Icon } from '@iconify-icon/solid';
import { platform } from '@tauri-apps/plugin-os';
import { createEffect, createMemo, createResource, For, Show } from 'solid-js';
import { commands, type PlaybackTranscodingCodec } from '~/bindings';
import { setPlaybackSetting, setPlaybackSettings, settingsStore } from '~/features/settings/service';
import { sessionStore } from '~/stores/SessionStore';
import Heading3 from '../common/text/Heading3';
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
  const isAndroid = platform() === 'android';
  const isWindows = platform() === 'windows';
  let outputDeviceSelect: HTMLSelectElement | undefined;
  const outputDeviceSource = () =>
    isWindows && sessionStore.playbackCapabilities.outputDeviceSelection && settingsStore.playback.useCustomOutput ? 'windows' : null;
  const [outputDevices] = createResource(outputDeviceSource, async () => {
    const result = await commands.playbackGetOutputDevices();
    if (result.status === 'error') {
      console.error(result.error);
      return [];
    }
    return result.data;
  });
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
  const customOutputDevices = createMemo(() => outputDevices() ?? []);
  const selectedOutputDeviceId = () => settingsStore.playback.outputDeviceId ?? '';
  const selectedOutputDeviceMissing = () => {
    const selected = settingsStore.playback.outputDeviceId;
    return !!selected && !(outputDevices() ?? []).some((device) => device.id === selected);
  };
  const setUseCustomOutput = async (enabled: boolean) => {
    await setPlaybackSettings({
      useCustomOutput: enabled,
      outputDeviceId: null,
    });
  };
  createEffect(() => {
    const selected = selectedOutputDeviceId();
    outputDevices();
    if (outputDeviceSelect && outputDeviceSelect.value !== selected) {
      outputDeviceSelect.value = selected;
    }
  });

  return (
    <SettingSection title='Playback'>
      <Show when={isWindows && sessionStore.playbackCapabilities.outputDeviceSelection}>
        <form class='flex flex-col gap-3'>
          <Heading3 class='mb-3'>Output Device</Heading3>
          <label class='flex items-center justify-between gap-4'>
            <span>Use Custom Output</span>
            <input
              type='checkbox'
              checked={settingsStore.playback.useCustomOutput}
              onChange={(event) => setUseCustomOutput(event.currentTarget.checked)}
            />
          </label>
          <Show when={settingsStore.playback.useCustomOutput}>
            <label class='flex items-center justify-between gap-4'>
              <span>Custom Output</span>
              <select
                ref={(element) => {
                  outputDeviceSelect = element;
                }}
                class='border-primary-border bg-primary-surface text-primary-text max-w-[min(26rem,60vw)] rounded border px-2 py-1 text-sm'
                value={selectedOutputDeviceId()}
                onChange={(event) => setPlaybackSetting('outputDeviceId', event.currentTarget.value.trim() || null)}
              >
                <option value=''>System Default</option>
                <Show when={selectedOutputDeviceMissing()}>
                  <option value={settingsStore.playback.outputDeviceId ?? ''}>Unavailable output</option>
                </Show>
                <For each={customOutputDevices()}>{(device) => <option value={device.id}>{device.name}</option>}</For>
              </select>
            </label>
          </Show>
        </form>
      </Show>
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

      <Heading3 class='mt-1'>Raw Stream</Heading3>
      <form class='flex flex-col gap-3'>
        <Show when={isAndroid} fallback={<p class='text-secondary-text italic'>[ nothing to show. ]</p>}>
          <label class='flex items-center justify-between gap-4 py-1'>
            <span>Use Transcoding Stream on metered networks</span>
            <input
              type='checkbox'
              checked={settingsStore.playback.meteredNetworkTranscodingEnabled}
              onChange={(event) => setPlaybackSetting('meteredNetworkTranscodingEnabled', event.currentTarget.checked)}
            />
          </label>
        </Show>
      </form>

      <Heading3 class='mt-1'>Transcoding Stream</Heading3>
      <form class='flex flex-col gap-3'>
        <label class='flex items-center justify-between gap-4'>
          <span>Stream Bitrate Limit</span>
          <select
            class='border-primary-border bg-primary-surface text-primary-text rounded border px-2 py-1 text-sm'
            value={settingsStore.playback.transcodingBitrateLimit}
            onChange={(event) => setPlaybackSetting('transcodingBitrateLimit', parseStreamBitrateLimit(event.currentTarget.value))}
          >
            <option value={96}>96 kbps</option>
            <option value={128}>128 kbps</option>
            <option value={192}>192 kbps</option>
            <option value={320}>320 kbps</option>
          </select>
        </label>
        <label class='flex items-center justify-between gap-4'>
          <span>Use Custom Codec (unstable)</span>
          <input
            type='checkbox'
            checked={settingsStore.playback.useCustomTranscodingCodec}
            onChange={(event) => setPlaybackSetting('useCustomTranscodingCodec', event.currentTarget.checked)}
          />
        </label>
        <label class='flex items-center justify-between gap-4'>
          <span>Transcoding Codec</span>
          <select
            class='border-primary-border bg-primary-surface text-primary-text rounded border px-2 py-1 text-sm disabled:opacity-70'
            disabled={!settingsStore.playback.useCustomTranscodingCodec}
            value={selectedTranscodingCodec()}
            onChange={(event) => setPlaybackSetting('transcodingCodec', event.currentTarget.value as PlaybackTranscodingCodec)}
          >
            <For each={transcodingCodecOptions()}>{(codec) => <option value={codec}>{TRANSCODING_CODEC_LABELS[codec]}</option>}</For>
          </select>
        </label>
      </form>
    </SettingSection>
  );
}

export default PlaybackSettings;
