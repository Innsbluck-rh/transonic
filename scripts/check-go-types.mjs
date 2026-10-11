import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = fileURLToPath(new URL('..', import.meta.url));
const generatedFile = join(repositoryRoot, 'server', 'internal', 'realtime', 'connect_types_gen.go');

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: repositoryRoot,
    encoding: 'utf8',
    shell: false,
  });

  if (result.error) {
    throw new Error(`Could not run ${command}: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(result.stderr?.trim() || `${command} failed (status ${result.status}, signal ${result.signal}).`);
  }

  return result.stdout;
}

function checkGoTypes() {
  const workDir = mkdtempSync(join(tmpdir(), 'transonic-go-types-'));
  try {
    const candidateFile = join(workDir, 'connect_types_gen.go');
    run('typeshare', [
      '--lang',
      'go',
      '--config-file',
      join(repositoryRoot, 'typeshare.toml'),
      '--output-file',
      candidateFile,
      join(repositoryRoot, 'src-tauri', 'src', 'models'),
    ]);
    const expected = run('gofmt', [candidateFile]);
    const actual = readFileSync(generatedFile, 'utf8');

    if (expected.replace(/\r\n/g, '\n') !== actual.replace(/\r\n/g, '\n')) {
      throw new Error('server/internal/realtime/connect_types_gen.go is out of sync. Regenerate it with: pnpm go-types:export');
    }
    console.log('connect_types_gen.go is in sync with the Rust source.');
  } finally {
    rmSync(workDir, { recursive: true, force: true });
  }
}

try {
  checkGoTypes();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
