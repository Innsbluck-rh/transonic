import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ANDROIDX_MEDIA_REF = '7ce3aa2619b19009e4799319b4dd694a4a7577df'; // Media3 1.10.0
const LIBFLAC_REPOSITORY = 'https://github.com/xiph/flac.git';
const LIBFLAC_REF = 'b430c3a58b64b70642ab5c72c36084dd4083d165';

const repositoryRoot = resolve(fileURLToPath(new URL('..', import.meta.url)));
const media3Checkout = join(repositoryRoot, 'third_party', 'androidx-media');
const libflacCheckout = join(media3Checkout, 'libraries', 'decoder_flac', 'src', 'main', 'jni', 'libflac');

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? repositoryRoot,
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

function cloneLibflac() {
  if (existsSync(libflacCheckout) && readdirSync(libflacCheckout).length > 0) {
    console.error(`${libflacCheckout} exists but is not a Git checkout.`);
    console.error('Remove it or move it aside, then run this setup again.');
    process.exit(1);
  }

  mkdirSync(dirname(libflacCheckout), { recursive: true });
  run('git', ['clone', '--filter=blob:none', LIBFLAC_REPOSITORY, libflacCheckout]);
}

run('git', ['submodule', 'update', '--init', 'third_party/androidx-media']);

if (!isGitCheckout(media3Checkout)) {
  console.error(`${media3Checkout} is not a Git checkout. Check the androidx-media submodule.`);
  process.exit(1);
}

ensureClean(media3Checkout, 'androidx-media');
run('git', ['-C', media3Checkout, 'checkout', '--detach', ANDROIDX_MEDIA_REF]);

if (!isGitCheckout(libflacCheckout)) {
  cloneLibflac();
}

ensureClean(libflacCheckout, 'libflac');
run('git', ['-C', libflacCheckout, 'fetch', '--filter=blob:none', 'origin']);
run('git', ['-C', libflacCheckout, 'checkout', '--detach', LIBFLAC_REF]);

console.log('Media3 FLAC decoder sources are ready.');
