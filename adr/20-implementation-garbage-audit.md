<!--
author:   Codex
date:     2026-06-20 22:27
version:  0.0.1
-->

## 概要

transonic 全体の実装に含まれる「ゴミ」を調査した記録です。

ここでの「ゴミ」は、未使用ファイルや生成物という意味ではありません。アプリの実装をより短く、見通しよく、単純にするうえで邪魔になる、実装上の枝葉、迂回、責務の重複、正本の分裂、明示されていない abandoned 状態のことです。

大きさは、消せるコード量と、問題が複数領域へ広がっている深さで判断しました。

## 分類

本 ADR では、各発見事項の「該当分類」を以下の意味で使います。分類は調査範囲を示すためのものであり、重要度順ではありません。

- A1: フロントエンド / スタイル
- A2: フロントエンド / コンポーネント
- A3: フロントエンド / テスト
- A4: フロントエンド / その他
- B1: Rustバックエンド / 再生機能(windows)
- B2: Rustバックエンド / 再生機能(android)
- B3: Rustバックエンド / 設定系
- B4: Rustバックエンド / フロントとの通信
- B5: Rustバックエンド / テスト
- B6: Rustバックエンド / その他
- C1: プロジェクト / pnpmパッケージ
- C2: プロジェクト / cargoパッケージ
- C3: プロジェクト / dev,build環境
- C4: プロジェクト / その他

## 大きいゴミ

### 1. playback command の IPC 契約が実態と合っていない

該当分類: A2: フロントエンド / コンポーネント、A4: フロントエンド / その他、B4: Rustバックエンド / フロントとの通信

- `src-tauri/src/commands/playback.rs`
- `src/features/playback/usePlayback.ts`
- `src/features/playback/service.ts`
- `src/components/common/list/song/SongList.tsx`
- `src/components/common/list/song/QueueList.tsx`

多くの playback command は `Result<(), String>` を返す形ですが、実際には `tauri::async_runtime::spawn` または `spawn_blocking` で処理を投げ、すぐ `Ok(())` を返しています。実処理の失敗は background task 内で log に流れるだけです。

一方でフロント側は `hasPlaybackCommandError()` を通し、失敗時に state refresh する構造を持っています。しかし command がほぼ失敗を返さないため、この error path は見かけ上の契約になっています。

さらに `SongList.tsx` と `QueueList.tsx` は `usePlayback()` の facade を通らず、`commands.playbackSetQueue()` や `commands.playbackPlayQueueIndex()` を直呼びしています。つまり「playback操作は hook/service に集める」という構造と、「component から IPC を直接叩く」構造が併存しています。

これは単一のバグではなく、IPC 契約、フロント facade、エラー復旧、component 境界が同時に膨らむ原因です。`accepted` 型の command と、完了を返す command を分けるか、playback command を同期的に完了させるかを明示しない限り、形だけの失敗処理が増えます。

### 2. settings の正本が TypeScript と Rust に分裂している

該当分類: A3: フロントエンド / テスト、B3: Rustバックエンド / 設定系、B4: Rustバックエンド / フロントとの通信

- `src/features/settings/service.ts`
- `src-tauri/src/models/settings.rs`
- `src-tauri/src/app_settings.rs`
- `src-tauri/src/commands/settings.rs`
- `src/components/settings/PlaybackSettings.tsx`

設定の default、normalize、legacy migration、optimistic update、rollback、永続化後の副作用が TS と Rust の両方にあります。

例として `gaplessPlaybackEnabled` の default は TS 側では `false`、Rust 側では `true` です。~~`useCustomOutput` は TS 側と Rust model 側にありますが、生成済み `src/bindings.ts` には存在しません。`PlaybackOutputDevice.deviceName` も Rust/FE 実装にはありますが、binding にはありません。~~\
  (2026-06-21 17:01) 生成済み `src/bindings.ts` には `PlaybackSettings.useCustomOutput` と `PlaybackOutputDevice.deviceName` が存在するため、この根拠は取り下げます。

これは `false` を `true` に直す類の問題ではありません。問題は、設定の意味、初期値、旧形式変換、検証、反映先が複数箇所で同時に管理されていることです。設定項目が増えるたびに、TS service、Rust model、command 副作用、bindings、テスト fixture が増えます。

設定は Rust 側を正本にし、フロントは受け取った設定の表示と差分送信に寄せるほうが、実装量を大きく削れます。

### 3. remote Connect shared playback が local PlaybackStatus に偽装されている

該当分類: A3: フロントエンド / テスト、A4: フロントエンド / その他、B4: Rustバックエンド / フロントとの通信

- `src/features/playback/sharedPlaybackStatus.ts`
- `src/features/playback/usePlaybackStatus.ts`
- `src/components/common/dialog/PlaybackStreamInfoDialog.tsx`
- `src/features/playback/usePlayback.test.ts`
- `src/components/common/dialog/PlaybackStreamInfoDialog.test.tsx`

`ConnectSharedPlaybackState` は remote device の共有再生状態です。しかし現在は `mergePlaybackStatusWithSharedPlayback()` で local `PlaybackStatus` へ合成され、remote では存在しない diagnostics を `null` や `gapless: remote playback diagnostics unavailable` で埋めています。

その結果、UI は `PlaybackStatus` に見える値が local か remote かを別関数で判定する必要があります。テストもこの偽装を固定しています。

remote 状態と local 状態を同じ型に押し込めたことで、表示側は単純に見えますが、diagnostics の偽値、除外判定、テスト fixture が増えています。`LocalPlaybackStatus | RemoteSharedPlaybackStatus` のように表示状態を分けるほうがコード量を削れます。

### 4. QueueSyncGateway が production no-op のまま controller に入り込んでいる

該当分類: B4: Rustバックエンド / フロントとの通信、B5: Rustバックエンド / テスト、B6: Rustバックエンド / その他

- `src-tauri/src/playback/queue_sync.rs`
- `src-tauri/src/playback/controller.rs`
- `src-tauri/src/playback/mod.rs`

`QueueSyncGateway` は queue 変更時に呼ばれますが、production 実装は `NoopQueueSyncGateway` です。`create_playback_controller()` も全 platform で no-op を渡しています。

それにもかかわらず controller は `queue_sync` field を持ち、queue mutation の複数箇所で `sync_queue_state()` を呼びます。さらに controller test は mock queue sync を用意し、この no-op seam を前提にしたテストを持っています。

これは未使用ファイルではなく、まだ存在しない機能のための抽象が中央 controller に侵入している状態です。実装されていない Connect queue sync を先取りして、production path と test path の両方を太らせています。

### 5. Windows Symphonia backend が shim ではなく巨大な再生エンジンになっている

該当分類: B1: Rustバックエンド / 再生機能(windows)

- `src-tauri/src/playback/backend_shims/windows_symphonia.rs`
- `src-tauri/src/playback/backend_shims/windows_mf.rs`
- ~~`src-tauri/src/playback/backend.rs`~~\
  (2026-06-21 17:01) このファイルは現状存在しないため、対象から除外します。

`windows_symphonia.rs` は単に行数が多いのではありません。1ファイル内に、progressive HTTP download buffer、Symphonia `MediaSource`、codec registry、decode pump、seek、audio ring buffer、resampling/downmix、gapless stream router、prepared activation、worker thread、output device selection、CPAL stream callback、stream error classification が混在しています。

`backend_shims` という名前の境界にありますが、実際には Windows 再生実装の中核がほぼここに集まっています。責務を切るだけで、読み取り対象とテスト対象を大きく減らせます。

別件として、~~`src-tauri/src/playback/backend.rs` は全体がコメントアウトされた旧 backend です。~~\
  (2026-06-21 17:01) `src-tauri/src/playback/backend.rs` は現状存在しないため、この記述は取り下げます。`windows_mf.rs` は ADR 上で保留された MediaFoundation backend ですが、現行 export にはつながっていません。

### 6. Android playback の media slot state が三重管理されている

該当分類: B2: Rustバックエンド / 再生機能(android)、C3: プロジェクト / dev,build環境

- `src-tauri/src/playback/backend_shims/android.rs`
- `src-tauri/gen/android/app/src/main/java/com/innsb/transonic/playback/AndroidPlaybackPlugin.kt`
- `src-tauri/gen/android/app/src/main/java/com/innsb/transonic/playback/PlaybackService.kt`

Android playback では、Rust backend、Kotlin plugin host、Kotlin `PlaybackService` がそれぞれ current/prepared/transitioning media の状態を持っています。

`AndroidPlaybackPlugin.kt` には `ControllerMediaState` と `currentMediaState` / `preparedMediaState` / `transitioningMediaState` があります。`PlaybackService.kt` にも `PlaybackSlotState` と同名の状態があります。Rust 側は `prepared_generation` と `media_instance_id` を持ちます。

同じ slot lifecycle が複数層に分かれ、さらに `basePositionMs` の所有も分裂しています。Android bridge の都合はありますが、状態正本が複数あることで、gapless 遷移、position 計算、手動遷移、通知操作の枝が増えています。

### 7. OpenSubsonic client crate の response 型境界が未完了

該当分類: B6: Rustバックエンド / その他、C2: プロジェクト / cargoパッケージ

- `docs/opensubsonic-client-design.md`
- `src-tauri/crates/opensubsonic-client/src/api/browsing.rs`
- `src-tauri/crates/opensubsonic-client/src/api/lists.rs`
- `src-tauri/crates/opensubsonic-client/src/api/search.rs`
- `src-tauri/src/commands/browse/mod.rs`
- `src-tauri/src/commands/browse/artist.rs`
- `src-tauri/src/commands/browse/album.rs`
- `src-tauri/src/commands/json.rs`

設計文書では low-level client が OpenSubsonic/Subsonic の endpoint request/response を扱い、commands は Tauri 境界に留める方針です。

しかし現状の `opensubsonic-client` は browsing/lists/search などで payload を `serde_json::Value` として返しています。実際の `RawSong`、`RawAlbum`、`RawArtist`、`vec_or_single`、`stringish`、`opt_u32ish` などの互換パースは Tauri command 側にあります。

OpenSubsonic 公式ドキュメントも endpoint response と `subsonic-response` envelope を前提にしています。今の構造では envelope だけ client crate に入り、互換性の難しい部分が command 層へ漏れています。API 追加時に command 側の Raw 型と parser が増え続ける構造です。

参照:

- https://opensubsonic.netlify.app/docs/api-reference/
- https://opensubsonic.netlify.app/docs/endpoints/getopensubsonicextensions/

### 8. Connect shared playback の queue 意味論が Go / Rust / TypeScript に分裂している

該当分類: A4: フロントエンド / その他、B4: Rustバックエンド / フロントとの通信、C4: プロジェクト / その他

- `server/internal/realtime/hub.go`
- `src-tauri/src/playback/controller.rs`
- `src/features/playback/usePlayback.ts`
- `server/internal/realtime/hub_test.go`

`playNextQueueLen`、`currentIndex`、`insertAfterCurrent`、`moveQueueIndex`、`removeQueueIndex`、`playQueueIndex` の意味論が Go server、Rust controller、TS display helper に分かれています。

Go には `playNextStartIndex`、`clampedPlayNextQueueLen`、`adjustedPlayNextQueueLenForCurrentChange` があります。Rust controller にも同等の関数があります。TS 側にも `isPlayNextQueueIndex()` があります。

Connect server と local controller の両方が queue authority になる事情はありますが、共有 protocol/domain model がないため、同じ概念を三言語で維持しています。ここは修正ではなく、実装パターンとして重い枝です。

### 9. Android の authored code が `src-tauri/gen/android` に埋まっている

該当分類: C3: プロジェクト / dev,build環境

- `src-tauri/gen/android/app/src/main/java/com/innsb/transonic/playback/AndroidPlaybackPlugin.kt`
- `src-tauri/gen/android/app/src/main/java/com/innsb/transonic/playback/PlaybackService.kt`
- `src-tauri/gen/android/app/build.gradle.kts`
- `src-tauri/gen/android/settings.gradle`
- `scripts/setup-media3-flac.mjs`
- `scripts/setup-media3-ffmpeg.mjs`

これは「生成物が大きい」という話ではありません。Android の主要実装が `gen/android` 配下にあり、Media3 decoder 設定、third_party checkout、FFmpeg/FLAC setup script、Gradle module 追加が同じ領域に混在しています。

Tauri Android の制約で `gen/android` 配下に置く必要がある部分はあります。それでも、現在の見え方では生成された project tree と手書き runtime code の境界が弱く、調査・レビュー時に「どこが実装正本か」を判断しづらくなっています。

### 10. global CSS に component 固有 state が残っている

該当分類: A1: フロントエンド / スタイル

- `src/styles.css`
- `src/components/common/list/song/SongList.tsx`
- `src/components/common/list/song/QueueList.tsx`

`styles.css` の base layer に `.song-item`、`.song-item-leading`、`.song-item-title`、`.song-item-meta`、`.button-borderless`、`.home-surface-root` が入っています。

Tailwind utility と component 実装を中心にしている全体規則から見ると、song list の current/selected state が global stylesheet に置かれているのは外れています。見た目の仕様変更時に TSX と global CSS の両方を見る必要があり、component 単位で完結しません。

## 小さいゴミ

### pnpm package の未使用・配置違い

該当分類: C1: プロジェクト / pnpmパッケージ

`@testing-library/jest-dom`、`@vitest/browser-playwright`、`@vitest/coverage-v8`、`playwright`、JS 側の `@tauri-apps/plugin-opener` は現状の import から見る限り使われていません。

`jsdom` と `prettier-plugin-tailwindcss` は runtime `dependencies` ではなく `devDependencies` が自然です。

これは削れば済む浅いゴミです。大きな構造問題ではありません。

### 意味が弱いテストと重複 fixture

該当分類: A3: フロントエンド / テスト、B5: Rustバックエンド / テスト

`ConnectButton.test.tsx` の `expect(() => dispatchEvent(...)).not.toThrow()` 系は、UIの意味より「落ちない」ことを固定しています。

また、複数の FE test が `PlaybackStatus`、`ConnectSharedPlaybackState`、settings fixture をそれぞれ手で作っています。これは settings 正本分裂や remote/local status 偽装とつながっており、型が増えるたびにテスト側の枝も増えます。
