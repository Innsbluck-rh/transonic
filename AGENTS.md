# overview

- This is a music player app project compatible with the subsonic API.
- Written in Tauri v2 and Solid, most of the communication and processing will be implemented on the backend side with Rust or a native Shim using Kotlin/Swing plugins. This means the frontend's responsibilities are expected to be limited to displaying that backend data.
- Please be sure to refer to the latest (open)subsonic API documentation regarding API compatibility.
- When you creating ADR file, you MUST refer `adr/README.md` to understand what is preferred and what isn't.

# instructions

- READ THE LATEST CODE. ALWAYS, READ. THE. ACTUAL. CODE. You are supposed to find out potential information, issues, and solutions. You are NOT supposed to report what your thought from user's prompt or TODO list. Instead, READ. THE. ACTUAL. CODE.
- Always run test if you made any change in test files.
- 「筋がいい」っていう言い方クソむかつくのでやめろ　「正しい」とかにしてくれ

## FORBIDDEN SYNTAX (PowerShell)

- **NO Linux Redirection:** DO NOT use `< < EOF` or `<< 'PY'`. These are Linux-specific and cause syntax errors in PowerShell.
- **NO `sed`/`grep` in `pwsh`:** Unless explicitly calling `wsl sed ...`, do not use these commands directly in PowerShell. Use native PowerShell cmdlets (e.g., `Select-String`, `b replace`).
