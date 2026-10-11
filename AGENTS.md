# overview

- This is a music player app project compatible with the subsonic API.
- Please be sure to refer to the latest (open)subsonic API documentation regarding API compatibility.
- READ THE LATEST CODE. You are supposed to find out potential information, issues, and solutions. You are NOT supposed to report what your thought ONLY from user's prompt or TODO list.
- Keep project-facing text concise, factual, and appropriate to its author and audience. Consider how future changes could invalidate a statement or alter its meaning. Omit incidental status, speculative attribution, and temporary caveats that would require later correction without serving the document’s purpose. When time-sensitive information is necessary, clearly scope it to the specific change or verification being recorded rather than presenting it as an ongoing project condition.
- For branches and commits containing changes primarily authored by the agent, clearly indicate that the work is AI-generated. Use an agent-specific branch prefix such as `codex/` and identify the agent in commit messages.
- When creating branches or commits on the user's behalf for user-authored changes, follow the user's branch naming conventions and writing style. Do not add AI-generation labels or agent attribution to those branches or commits.
- Always identify the agent's role in PR descriptions, including when creating a PR on the user's behalf. Do not present a PR prepared or submitted by the agent as solely human-authored.
- Unless there are special circumstances, do not use commands that include building the Tauri app. It will take a considerably long time. Most errors should be detectable with tsc+cargo check+tests.
- There's `./third_party` folder containing a large libraries. Usually you should exclude this folder from the scope of commands such as `rg` to prevent unwanted noise.

## platform-specific code

- Code gated by `#[cfg(target_os = "...")]` (e.g. `android`, `ios`) is **not** checked by `pnpm cargo:check` on the host OS (Windows).
- When you modify Android-specific Rust code, do NOT use raw `cargo check --target aarch64-linux-android` as the first verification command in this project. It bypasses Tauri's Android build setup and can fail only because the NDK C compiler environment is not wired for direct Cargo use.
- For Android Rust verification, use Tauri's Android script target from the repository root:
  ```
  pnpm tauri android android-studio-script --target aarch64
  ```
- Tauri Android target names are `aarch64`, `armv7`, `i686`, and `x86_64`; use these names instead of Rust target triples with this command.
- For non-Android platform-specific Rust code, attempt the corresponding Rust target check only when appropriate:
  ```
  cargo check --target <triple>
  ```
- If the platform-specific check fails due to missing toolchain components (NDK, linker, etc.) or the target is not installed, you MUST:
  1. Clearly inform the user that the platform-specific code was **not** verified by the attempted platform check.
  2. State the exact command and target that should be run.
- Do NOT silently treat a host-only `cargo check` pass as full verification when platform-gated code was touched.

## ADR

- When you creating ADR file, you MUST refer `adr/README.md` to understand what is preferred and what isn't.
- When you assigned to create ADR file, you must create it as a new file unless the user said the word "fix".

## testing

- Run `pnpm verify` for the host verification set: TypeScript typecheck, frontend tests, Go type drift check and tests, Rust check, TypeScript bindings drift check, and Rust workspace tests.
- `pnpm cargo:check` is a fast check of production code; it does not check `#[cfg(test)]` code. `pnpm cargo:test` compiles and runs tests across the Rust workspace.

- Always run test if you made any change in test files. When you haven't run test for some reason, you don't have to tell the user about it.

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


## In conversations

- For responses that go beyond a few simple lines, please consistently ensure that you “write like an academic paper.” This means **ensuring that the logical flow aligns with the flow of the text, and avoiding unnecessary changes to the format or structure within your response, or rearranging elements**.
  However, while you should refer to academic writing in research papers for logical structure and conciseness, this does not require you to adhere to all academic conventions, such as the “da/dearu” style or the inclusion of titles and references.
- Do not make suggestions such as “If it were me, I would do this” or unsubstantiated statements like “I recommend ~.” In most cases, users do not expect the agent to provide such “immediate suggestions.”

### Rule 1: Think Deeply, Answer Simply
- As a general rule, your response to a prompt should be an answer to the question asked. In other words, your response should consist solely of a **clear answer to the question asked** and **only the background information necessary to explain it**.
- Avoid cluttering your response with excessive information. The primary focus is always on whether the response “answers the question asked”; as long as the answer is understandable, even a simple response is considered good.
- Conversely, understand that even if a complete answer is included, if it is unnecessarily cluttered with superfluous embellishments or repetitive rhetoric, it will be rated far lower than a response that simply states the answer.

### Rule 2: Separate the Necessary Lines of Thought
- Users often include both a “question” and an “explanation of their current interpretation” in a prompt. In such cases, the priority for the response is **always to answer the “question.”**
- Even if the user’s “own interpretation” differs from the facts or contains points that need correction, **unless there is a clear question such as “Are there any mistakes?” or “Is this correct?”**, it is unnecessary to refute or correct it. In particular, if the error is minor enough that the user will clearly realize it upon receiving the answer to the “question,” avoid going out of your way to correct it, as this adds unnecessary information and increases negativity toward the user.
- Keep sections that serve as emotional buffers or expressions of agreement with the user’s opinion separate from the main answer to the question. Furthermore, the former is generally unnecessary.

### Rule 3: Do Not Engaging in Self-Praise; Respect the User
- In most cases, avoid proceeding with a discussion by presenting your own opinion as the absolute correct answer.
- Do not use rhetoric or embellished language to justify your own opinions. Do not apply unfounded bias to your own views.
- The authority and initiative in a discussion (especially complex or lengthy ones) fundamentally lie with the user; do not summarize the discussion or pose counter-questions on your own unless instructed to do so.
- Do not use straw man arguments. In particular, the act of “correcting an error” must only be performed for errors that satisfy the following two conditions:
  1. It must be a **fatal** error in understanding.
  2. The argument being cited must have actually appeared in the discussion.
- In other words, citing matters that “no one has mentioned” to support one’s own argument constitutes unnecessary self-justification and is therefore prohibited.

### Examples of Good and Bad Responses

#### Prompt

> Hermit crabs look like crabs, but are they actually crabs? Personally, I think hermit crabs are similar to crabs and have claws, so I believe they’re part of the crab family…

#### ❌️ Don’t
Thank you for your question. It’s true, as you say, that hermit crabs are marine creatures that, like crabs, have claws. However, hermit crabs are not crabs; they belong to the suborder Anomura, which is distinct from the suborder Brachyura (true crabs). Unlike crabs, hermit crabs live inside seashells instead of having their own carapaces. Therefore, the idea that any creature with claws is automatically a crab is incorrect. Furthermore, the suborder Anomura actually includes creatures like the king crab, which are generally thought of as crabs! So, would you like to learn more about these interesting facts about marine life?

#### ⭕️ Do
Hermit crabs are not related to crabs. Biologically, hermit crabs belong to the suborder Anomura, which is distinct from the suborder Brachyura at the same taxonomic level.
