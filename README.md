# Transonic

Multi-platform music player for subsonic servers.

## icon gen

```
pnpm tauri icon ./public/icon.svg -o ./src-tauri/icons
```

## apk build for debug

```
pnpm tauri android build --apk --target aarch64
adb install -r ".\src-tauri\gen/android\app/build/outputs/apk/universal/release/app-universal-release.apk"
```