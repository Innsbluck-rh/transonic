import { commands, events, type AppSettings, type ConnectSharedPlaybackState } from '~/bindings';
import { settingsStore } from '~/features/settings/service';
import { connectStore, setConnectStore } from '~/stores/ConnectStore';
import { sessionStore } from '~/stores/SessionStore';

interface VersionResponse {
  version: string;
  protocolVersion: number;
}

interface CapabilityResponse {
  presence: boolean;
  sharedQueue: boolean;
  sharedPlayback: boolean;
  playbackTransfer: boolean;
}

let refreshGeneration = 0;

function monotonicNowMs() {
  return typeof performance === 'undefined' ? Date.now() : performance.now();
}

export function splitUrlHostAndPort(rawUrl: string | null) {
  const trimmed = rawUrl?.trim();
  if (!trimmed) {
    return { host: '', port: null as number | null };
  }

  try {
    const parsed = new URL(trimmed);
    const port = parsed.port ? Number(parsed.port) : null;
    parsed.pathname = '';
    parsed.search = '';
    parsed.hash = '';
    parsed.port = '';
    return {
      host: parsed.toString().replace(/\/$/, ''),
      port: Number.isInteger(port) && port !== null ? port : null,
    };
  } catch {
    return { host: trimmed, port: null };
  }
}

export function connectServerHostFromSubsonic(subsonicServerUrl: string | null) {
  return splitUrlHostAndPort(subsonicServerUrl).host;
}

export function normalizeConnectServerUrl(rawUrl: string | null, allowInsecure: boolean, connectServerPort?: number) {
  const trimmed = rawUrl?.trim();
  if (!trimmed) {
    return null;
  }

  let parsed: URL;
  try {
    parsed = new URL(trimmed);
  } catch {
    throw new Error('Connect Server URL is invalid.');
  }

  if (parsed.username || parsed.password) {
    throw new Error('Connect Server URL must not include credentials.');
  }

  if (parsed.protocol === 'http:') {
    const hostname = parsed.hostname.toLowerCase();
    const loopback = hostname === 'localhost' || hostname === '127.0.0.1' || hostname === '[::1]' || hostname === '::1';
    if (!loopback && !allowInsecure) {
      throw new Error('LAN HTTP Connect Server requires the insecure HTTP setting.');
    }
  } else if (parsed.protocol !== 'https:') {
    throw new Error('Connect Server URL must start with http:// or https://.');
  }

  parsed.pathname = parsed.pathname.replace(/\/+$/, '');
  if (typeof connectServerPort === 'number' && Number.isInteger(connectServerPort) && connectServerPort > 0 && connectServerPort <= 65535) {
    parsed.port = String(connectServerPort);
  }
  parsed.search = '';
  parsed.hash = '';
  return parsed.toString().replace(/\/$/, '');
}

export function resolveConnectServerUrl(connect: AppSettings['connect'], subsonicServerUrl: string | null) {
  if (!connect.enabled) {
    return null;
  }

  if (!connect.useSubsonicServerHost) {
    return normalizeConnectServerUrl(
      connect.connectServerHost ?? null,
      connect.allowInsecureConnectServer === true,
      connect.connectServerPort || 4747
    );
  }

  if (!subsonicServerUrl) {
    throw new Error('Active Subsonic Server URL is unavailable.');
  }

  let parsed: URL;
  try {
    parsed = new URL(subsonicServerUrl);
  } catch {
    throw new Error('Subsonic Server URL is invalid.');
  }

  parsed.pathname = '';
  parsed.search = '';
  parsed.hash = '';
  parsed.port = String(connect.connectServerPort || 4747);
  return normalizeConnectServerUrl(parsed.toString(), connect.allowInsecureConnectServer === true || parsed.protocol === 'http:');
}

export async function refreshConnectServerStatus() {
  const generation = ++refreshGeneration;
  if (!settingsStore.connect.enabled) {
    setConnectStore({
      status: 'disabled',
      message: null,
      version: null,
      protocolVersion: null,
      capabilities: null,
      sharedPlayback: null,
      sharedPlaybackReceivedAtMs: null,
    });
    return;
  }

  setConnectStore('status', 'checking');
  setConnectStore('message', null);

  try {
    const baseUrl = resolveConnectServerUrl(settingsStore.connect, sessionStore.activeSession?.normalizedServerUrl ?? null);
    if (!baseUrl) {
      return;
    }

    const [healthResponse, versionResponse, capabilitiesResponse] = await Promise.all([
      fetch(`${baseUrl}/healthz`),
      fetch(`${baseUrl}/version`),
      fetch(`${baseUrl}/v1/capabilities`),
    ]);

    if (generation !== refreshGeneration) {
      return;
    }

    if (!healthResponse.ok || !versionResponse.ok || !capabilitiesResponse.ok) {
      throw new Error('Connect server health check failed.');
    }

    const version = (await versionResponse.json()) as VersionResponse;
    const capabilities = (await capabilitiesResponse.json()) as CapabilityResponse;
    setConnectStore({
      status: 'available',
      message: null,
      version: version.version,
      protocolVersion: version.protocolVersion,
      capabilities,
    });
  } catch (error) {
    if (generation !== refreshGeneration) {
      return;
    }

    setConnectStore({
      status: 'unavailable',
      message: error instanceof Error ? error.message : String(error),
      version: null,
      protocolVersion: null,
      capabilities: null,
      sharedPlayback: null,
      sharedPlaybackReceivedAtMs: null,
    });
  }
}

export async function refreshConnectRuntimeStatus() {
  try {
    const status = await commands.connectGetRuntimeStatus();
    setConnectStore('runtime', {
      enabled: status.enabled,
      connected: status.connected,
      message: status.message,
      deviceId: status.deviceId,
      seq: status.seq,
    });
    if (!status.connected) {
      setConnectStore('devices', []);
      setConnectStore('sharedPlayback', null);
      setConnectStore('sharedPlaybackReceivedAtMs', null);
    }
  } catch (error) {
    setConnectStore('runtime', {
      enabled: false,
      connected: false,
      message: error instanceof Error ? error.message : String(error),
      deviceId: null,
      seq: 0,
    });
    setConnectStore('devices', []);
    setConnectStore('sharedPlayback', null);
    setConnectStore('sharedPlaybackReceivedAtMs', null);
  }
}

export async function refreshConnectDevices() {
  const devices = await commands.connectGetDevices();
  setConnectStore('devices', devices);
}

function applyConnectSharedPlayback(sharedPlayback: ConnectSharedPlaybackState | null) {
  if (sharedPlayback === null) {
    if (connectStore.runtime.connected && connectStore.sharedPlayback !== null) {
      return;
    }
    setConnectStore('sharedPlayback', null);
    setConnectStore('sharedPlaybackReceivedAtMs', null);
    return;
  }

  if (connectStore.sharedPlayback && sharedPlayback.seq < connectStore.sharedPlayback.seq) {
    return;
  }

  setConnectStore('sharedPlayback', sharedPlayback);
  setConnectStore('sharedPlaybackReceivedAtMs', monotonicNowMs());
}

export async function refreshConnectSharedPlayback() {
  applyConnectSharedPlayback(await commands.connectGetSharedPlayback());
}

export async function startConnectStateSync() {
  const unlistenDevices = await events.connectDevicesUpdated.listen((event) => {
    setConnectStore('devices', event.payload.devices);
  });
  const unlistenSharedPlayback = await events.connectSharedPlaybackUpdated.listen((event) => {
    applyConnectSharedPlayback(event.payload.sharedPlayback);
  });
  try {
    await Promise.all([refreshConnectDevices(), refreshConnectSharedPlayback()]);
  } catch (error) {
    unlistenDevices();
    unlistenSharedPlayback();
    throw error;
  }
  return () => {
    unlistenDevices();
    unlistenSharedPlayback();
  };
}
