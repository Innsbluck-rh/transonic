# overview

- This is a music player app project compatible with the subsonic API.
- Please be sure to refer to the latest (open)subsonic API documentation regarding API compatibility.
- READ THE LATEST CODE. You are supposed to find out potential information, issues, and solutions. You are NOT supposed to report what your thought ONLY from user's prompt or TODO list.
- When you assigned to "research"(「調査」), "consider"(「検討」) or "investigate", you MUST NOT change anything.
- Unless there are special circumstances, do not use commands that include building the Tauri app. It will take a considerably long time. Most errors should be detectable with tsc+cargo check+tests.
- When executing cargo-related commands, please use `pnpm cargo:check` and `pnpm cargo:test` whenever possible.

## platform-specific code

- Code gated by `#[cfg(target_os = "...")]` (e.g. `android`, `ios`) is **not** checked by `pnpm cargo:check` on the host OS (Windows).
- When you modify platform-specific code, you MUST attempt to verify it with the corresponding target:
  ```
  cargo check --target <triple>          # e.g. aarch64-linux-android
  ```
- If the check fails due to missing toolchain components (NDK, linker, etc.) or the target is not installed, you MUST:
  1. Clearly inform the user that the platform-specific code was **not** verified by cargo check.
  2. State the exact command and target triple the user should run.
- Do NOT silently treat a host-only `cargo check` pass as full verification when platform-gated code was touched.

## ADR

- When you creating ADR file, you MUST refer `adr/README.md` to understand what is preferred and what isn't.
- When you assigned to create ADR file, you must create it as a new file unless the user said the word "fix".

## testing

- Always run test if you made any change in test files. When you haven't run test for some reason, you don't have to tell the user about it.

## changes / git

- Usually, you don't have to care about the status of version control (e.g.: git.)
- Don't ask the user about what have been changed while you're working (may be done by the user), unless it does really matter to your assignment.

## FORBIDDEN SYNTAX (PowerShell)

- **NO Linux Redirection:** DO NOT use `< < EOF` or `<< 'PY'`. These are Linux-specific and cause syntax errors in PowerShell.
- **NO `sed`/`grep` in `pwsh`:** Unless explicitly calling `wsl sed ...`, do not use these commands directly in PowerShell. Use native PowerShell cmdlets (e.g., `Select-String`, `b replace`).

## Encoding and Line Break Codes

- Path retrieval and file operations should **always** be performed using PowerShell.
- When handling file and folder paths or the contents of text files, use the following character encodings and line break codes:x
  - Display of paths and text on the PowerShell console: UTF-8, LF.
  - CSV files: UTF-8 with BOM, CRLF.
  - Text-based files such as Markdown, YAML, TOML, and Text: UTF-8 without BOM, LF.
  - PowerShell 5.x source files: UTF-8 with BOM, LF.
  - PowerShell 7.x source files: UTF-8 without BOM, LF.
