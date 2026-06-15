import { spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ANDROIDX_MEDIA_REF = '7ce3aa2619b19009e4799319b4dd694a4a7577df'; // Media3 1.10.0
const FFMPEG_REPOSITORY = 'https://github.com/FFmpeg/FFmpeg.git';
const FFMPEG_REF = 'n6.0';
const ENABLED_DECODERS = ['alac', 'vorbis', 'opus'];
const ANDROID_ABI = '24';

const repositoryRoot = resolve(fileURLToPath(new URL('..', import.meta.url)));
const androidRoot = join(repositoryRoot, 'src-tauri', 'gen', 'android');
const media3Checkout = join(repositoryRoot, 'third_party', 'androidx-media');
const ffmpegModulePath = join(media3Checkout, 'libraries', 'decoder_ffmpeg', 'src', 'main');
const ffmpegCheckout = join(ffmpegModulePath, 'jni', 'ffmpeg');
const toolShimDir = join(tmpdir(), 'transonic-media3-ffmpeg-tools');

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? repositoryRoot,
    env: options.env ? { ...process.env, ...options.env } : process.env,
    shell: false,
    stdio: 'inherit',
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function capture(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? repositoryRoot,
    encoding: 'utf8',
    shell: false,
  });

  if (result.status !== 0) {
    return null;
  }

  return result.stdout.trim();
}

function isGitCheckout(path) {
  return capture('git', ['-C', path, 'rev-parse', '--is-inside-work-tree']) === 'true';
}

function ensureClean(path, label) {
  const status = capture('git', ['-C', path, 'status', '--short']);
  if (status) {
    console.error(`${label} has local changes. Commit, stash, or reset them before running this setup.`);
    console.error(status);
    process.exit(1);
  }
}

function cloneFfmpeg() {
  if (existsSync(ffmpegCheckout) && readdirSync(ffmpegCheckout).length > 0) {
    console.error(`${ffmpegCheckout} exists but is not a Git checkout.`);
    console.error('Remove it or move it aside, then run this setup again.');
    process.exit(1);
  }

  mkdirSync(dirname(ffmpegCheckout), { recursive: true });
  run('git', ['clone', '--filter=blob:none', '--branch', FFMPEG_REF, '--depth', '1', FFMPEG_REPOSITORY, ffmpegCheckout]);
}

function ensureBash() {
  if (capture('bash', ['--version'])) {
    return;
  }

  const message =
    process.platform === 'win32'
      ? 'Media3 FFmpeg setup on Windows requires bash, such as Git Bash, available on PATH.'
      : 'Media3 FFmpeg setup requires bash on PATH.';
  console.error(message);
  process.exit(1);
}

function bashPath(path) {
  if (process.platform !== 'win32') {
    return path;
  }

  const converted = capture('bash', ['-lc', 'cygpath -u "$1"', '_', path])
    ?.split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .at(-1);
  if (!converted) {
    console.error(`Failed to convert Windows path for bash: ${path}`);
    process.exit(1);
  }
  return converted;
}

function ensureMakeForBash() {
  if (capture('bash', ['-lc', 'command -v make'])) {
    return null;
  }
  if (!capture('bash', ['-lc', 'command -v mingw32-make'])) {
    console.error('Media3 FFmpeg setup requires make on PATH. Install make, or make mingw32-make available as make.');
    process.exit(1);
  }

  mkdirSync(toolShimDir, { recursive: true });
  const shimPath = join(toolShimDir, 'make');
  writeFileSync(shimPath, '#!/usr/bin/env bash\nexec mingw32-make "$@"\n', 'utf8');
  chmodSync(shimPath, 0o755);
  return toolShimDir;
}

function readAndroidLocalProperties() {
  const localPropertiesPath = join(androidRoot, 'local.properties');
  if (!existsSync(localPropertiesPath)) {
    return {};
  }

  return Object.fromEntries(
    readFileSync(localPropertiesPath, 'utf8')
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith('#'))
      .map((line) => {
        const separator = line.indexOf('=');
        if (separator < 0) {
          return [line, ''];
        }
        return [
          line.slice(0, separator).trim(),
          line
            .slice(separator + 1)
            .trim()
            .replace(/\\:/g, ':')
            .replace(/\\\\/g, '\\'),
        ];
      })
  );
}

function newestInstalledNdk(sdkDir) {
  const ndkRoot = join(sdkDir, 'ndk');
  if (!existsSync(ndkRoot)) {
    return null;
  }

  const candidates = readdirSync(ndkRoot)
    .map((name) => join(ndkRoot, name))
    .filter((path) => existsSync(join(path, 'toolchains', 'llvm', 'prebuilt')));
  const preferred = candidates.filter((path) => /[\\/]26\./.test(path));

  return (preferred.length > 0 ? preferred : candidates).sort().at(-1) ?? null;
}

function resolveNdkPath() {
  if (process.env.NDK_PATH) {
    return process.env.NDK_PATH;
  }
  if (process.env.ANDROID_NDK_HOME) {
    return process.env.ANDROID_NDK_HOME;
  }
  if (process.env.ANDROID_NDK_ROOT) {
    return process.env.ANDROID_NDK_ROOT;
  }

  const localProperties = readAndroidLocalProperties();
  if (localProperties['ndk.dir']) {
    return localProperties['ndk.dir'];
  }
  if (localProperties['sdk.dir']) {
    return newestInstalledNdk(localProperties['sdk.dir']);
  }

  return null;
}

function hostPlatform() {
  if (process.env.HOST_PLATFORM) {
    return process.env.HOST_PLATFORM;
  }

  switch (process.platform) {
    case 'linux':
      return 'linux-x86_64';
    case 'darwin':
      return 'darwin-x86_64';
    case 'win32':
      return 'windows-x86_64';
    default:
      return null;
  }
}

ensureBash();

run('git', ['submodule', 'update', '--init', 'third_party/androidx-media']);

if (!isGitCheckout(media3Checkout)) {
  console.error(`${media3Checkout} is not a Git checkout. Check the androidx-media submodule.`);
  process.exit(1);
}

ensureClean(media3Checkout, 'androidx-media');
run('git', ['-C', media3Checkout, 'checkout', '--detach', ANDROIDX_MEDIA_REF]);

if (!isGitCheckout(ffmpegCheckout)) {
  cloneFfmpeg();
}

ensureClean(ffmpegCheckout, 'FFmpeg');
run('git', ['-C', ffmpegCheckout, 'fetch', '--depth', '1', 'origin', FFMPEG_REF]);
run('git', ['-C', ffmpegCheckout, 'checkout', '--detach', FFMPEG_REF]);

const ndkPath = resolveNdkPath();
if (!ndkPath) {
  console.error('Android NDK was not found. Set NDK_PATH, ANDROID_NDK_HOME, ANDROID_NDK_ROOT, or sdk.dir/ndk.dir in Android local.properties.');
  process.exit(1);
}

const resolvedHostPlatform = hostPlatform();
if (!resolvedHostPlatform) {
  console.error('Unsupported host platform. Set HOST_PLATFORM manually for the Android NDK prebuilt toolchain.');
  process.exit(1);
}

const makeShimPath = ensureMakeForBash();

run(
  'bash',
  [
    bashPath(join(ffmpegModulePath, 'jni', 'build_ffmpeg.sh')),
    bashPath(ffmpegModulePath),
    bashPath(ndkPath),
    resolvedHostPlatform,
    ANDROID_ABI,
    ...ENABLED_DECODERS,
  ],
  {
    cwd: join(ffmpegModulePath, 'jni'),
    env: makeShimPath ? { PATH: `${makeShimPath};${process.env.PATH ?? ''}` } : undefined,
  }
);

console.log(`Media3 FFmpeg decoder sources are ready with decoders: ${ENABLED_DECODERS.join(', ')}.`);
