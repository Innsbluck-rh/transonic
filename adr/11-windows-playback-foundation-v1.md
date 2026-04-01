<!--
author: Codex
date: 2026-04-01 09:35
version: 0.0.1
-->

## 概要

- ~~ADR10 から、再生状態の正本を Rust backend に置くこと、queue の現在位置を song ID ではなく index で扱うこと、v1 は Windows 実装のみを持つこと、server 報告とローカル再生制御を分離することを読み取りました。~~
  (2026-04-01 17:27) この記述のうち、Windows 実装のみを持つという部分は v1 を Windows 固定と受け取られやすいため修正しました。2026-04-01 時点では Android の再生方式を別途選定していますが、再生 backend は Windows、Android、そのほかの OS に広がりうる前提で扱います。
- 実装した内容は、`src-tauri/src/playback/` の `PlaybackController`、Windows 向け backend、再生 command 群、playback モデル型、TypeScript bindings への API 追加です。
- 実装しなかった内容は、frontend からの UI 接続、自動 next、`reportPlayback` / `scrobble` 実送信、`save/getPlayQueue(ByIndex)` 実送信です。

## 重要な意思決定

- 再生方式は Rust in-process を採用し、Windows backend に `rodio` を採用しました。
- `rodio::OutputStream` が `Send` ではないため、backend は専用ワーカースレッドを立て、controller 側はチャネル経由で制御する構成にしました。
- 音源取得は `stream` の `format=raw` を先に試し、失敗時は `format` 指定なしへフォールバックする実装にしました。
- `timeOffset` は `capability_matrix.transcode_offset` が真の場合のみ付与し、未対応サーバーでは seek 指定を 0 扱いにしました。
- server 連携は `PlaybackReporter` と `QueueSyncGateway` の境界だけを追加し、今回は Noop 実装を注入しました。
- ~~非 Windows では `UnsupportedPlaybackBackend` を使い、コード構造を保ったままビルド互換を維持しました。~~
  (2026-04-01 17:27) これは 2026-04-01 09:35 時点の実装状況でした。非 Windows を恒久的に `UnsupportedPlaybackBackend` とする判断ではなく、Windows、Android、そのほかの OS の backend を追加しうる前提に修正しました。

## playback API

- すべての command は `PlaybackStatus` を返します。
- `playback_get_state`: 現在の playback 状態を取得します。
- `playback_set_queue(PlaybackSetQueueRequest)`: queue と currentIndex を設定します。重複 song ID を許容します。
- `playback_play`: currentIndex の曲をロードして再生します。未指定時は 0 番目を採用します。
- `playback_pause`: 再生中の曲を一時停止します。
- `playback_stop`: 再生を停止し、現在曲の position を 0 に戻します。
- `playback_seek(PlaybackSeekRequest)`: 指定 position に移動します。`transcodeOffset` 非対応時は実効値 0 になります。
- `playback_next`: currentIndex を +1 して遷移します。末尾ではエラーになります。
- `playback_prev`: currentIndex を -1 して遷移します。先頭ではエラーになります。
