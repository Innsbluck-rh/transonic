<!--
author:  innsbluck / Claude
date:    2026-04-05 15:22
version: 0.0.1
-->

## 概要

- アプリ再起動後もキューと再生位置を復帰できるよう、playback state の永続化を行うことに決定した。
- キューへの部分操作（"次に再生"、"末尾に追加"）を `PlaybackController` に追加することに決定した。
- 永続化ファイルは `playback-state.json` とし、ファイル名にバージョン接尾辞は付けない。

## 今回決めたこと

### playback state を `playback-state.json` として永続化する

- 保存先は `app_config_dir` 配下の `playback-state.json` とした。
- 保存対象は、キュー（`Vec<SongResponse>`）、`current_index`、`current_position_ms`、および対象プロファイルの `profile_id` とした。
- `playing_state`、`interrupt_reason`、`pending_seek_position_ms`、`error` は一時的な実行時状態であるため、保存対象に含めない。
- ファイル内に `version` フィールドを持たせ、初期値は `1` とする。

### ファイル名にバージョン接尾辞を付けない

- 既存の `profiles-v1.json` はファイル名と内部フィールドの両方にバージョンを持つ二重管理になっている。playback state ではこれを踏襲しない。
- transonic は未公開のアプリであり、初回リリースの時点で volume、shuffle、repeat、original queue（シャッフル前のキュー）を含む主要な再生機能が揃っている前提で開発を進めている。そのため `version: 1` のスキーマがほぼ最終形であり、リリース後に非互換な変更が発生する可能性は低い。
- リリース前の開発段階ではユーザーは開発者のみであるため、スキーマ変更による被害は発生しない。
- 内部の `version` フィールドは、万が一の非互換変更時に「読めないバージョンなら空状態で起動する」という安全弁として機能させる。

### 永続化の責務は Rust backend に置く

- ADR 10 で決定した「再生状態の正本は Rust backend に置く」の原則に従い、永続化も Rust 側で行う。
- 既存の `QueueSyncGateway` トレイトは server 同期を意図した設計であるため、ローカル永続化の経路としては使用しない。ローカル永続化は `PlaybackController` から独立した保存経路として設計する。
- 保存の JSON ファイル I/O は `profiles.rs` と同じパターン（`serde_json::to_string_pretty` + `fs::File::create`）を採用する。

### 保存タイミング

- キュー内容の変化（set_queue、insert、append、reset）が発生した時点で保存する。
- `current_index` の変化（next、prev、play_queue_index）が発生した時点で保存する。
- `current_position_ms` は再生中に高頻度で変化するため、毎回の変化では保存しない。pause、stop、およびアプリ終了時に保存する。
- アプリ終了時の保存は Tauri の `RunEvent::ExitRequested` または `RunEvent::Exit` を利用する。

### 起動時の復帰

- `bootstrap_app_state` でプロファイル復帰後に `playback-state.json` を読み込む。
- 保存されている `profile_id` と現在のアクティブプロファイルが一致する場合のみ復帰する。一致しない場合は読み捨てる。
- 復帰時は `playing_state` を `Stopped` に設定する。自動再生は行わない。
- ファイルが存在しない、またはパースに失敗した場合は空状態で起動する。

### playback state はプロファイルに紐付く

- `playback-state.json` に `profile_id` を含め、保存元のプロファイルを識別可能にする。
- 曲 ID はサーバー固有であるため、異なるサーバーのキューを復帰しても意味がない。`profile_id` の一致確認はこの前提に基づく。

### UI 設定（グリッド/リスト、サイドバー幅など）はこのファイルに含めない

- `playback-state.json` は再生ドメインの状態のみを扱う。
- カラーテーマは既に `localStorage` で永続化されている。browse mode やグリッド/リスト切替、サイドバー幅などの UI 設定は再生ドメインとは更新頻度もデータ所有者も異なるため、別の経路で永続化する。

### "次に再生" と "末尾に追加" を PlaybackController に追加する

- `PlaybackController` にキューの部分操作メソッドを追加する。
- "次に再生"（insert after current）は `current_index` の直後に曲を挿入する。`current_index` 自体は変化しない。
- "末尾に追加"（append to queue）はキュー末尾に曲を追加する。`current_index` は変化しない。
- いずれの操作も、再生中の曲を中断せずにキューのみを変更する。
- 対応する Tauri コマンドを追加し、frontend の `usePlayback` からアクション関数として公開する。

### キューが空の状態での部分操作

- キューが空の状態で insert after current または append to queue が呼ばれた場合、追加された曲をキューに設定し `current_index` を `0` にする。自動再生は行わない。

## 判断理由

### 永続化がないとアプリ再起動でキューが失われる

- 現状は `PlaybackController` が `PlaybackStatus::empty()` で初期化され、`QueueSyncGateway` は `NoopQueueSyncGateway` が使用されている。アプリを閉じると再生状態がすべて消失する。
- 音楽プレイヤーとして、キューの喪失はユーザー体験の大きな損失にあたる。

### キューの全置換しか手段がないのは音楽プレイヤーとして不足している

- 現状はすべての再生操作（album 再生、folder album 再生、songs 再生）が `set_queue` でキュー全体を置換する。
- ユーザーが組み立てたキューに曲を追加する手段がなく、"次に再生" や "末尾に追加" といった基本的な操作ができない。

### QueueSyncGateway をローカル永続化に転用しない理由

- `QueueSyncGateway` は ADR 10 で「queue の server 同期は capability ベースで行う」と決定された server 同期のための境界として設計されている。
- ローカルファイルへの永続化は server 同期とは目的もタイミングも異なるため、同一のトレイトに相乗りさせると責務が混在する。
- `sync_queue` の呼び出し箇所は `set_queue` と `reset` のみだが、ローカル永続化では `next`、`prev`、`pause`、`stop` でも保存が必要になる。呼び出しタイミングの要件が異なる。

### 復帰時に自動再生しない理由

- アプリ起動時に予期せず音声が再生されるのは、ユーザー体験として許容しがたい。
- `Stopped` 状態で復帰し、ユーザーが明示的に再生を開始する形にする。
