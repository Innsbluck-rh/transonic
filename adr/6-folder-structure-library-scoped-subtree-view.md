<!--
author: Codex
date: 2026-03-30 09:53
version: 0.0.1
-->

## 概要

- `Folder Structure` を「library-scoped subtree album view」として実装した。
- 実装の中心は Rust backend 側で、library ごとの root 取得、subtree 走査、song からの album 集約を command 層で扱う形にした。
- 実装の途中で `commands.rs` が肥大化したため、parser と folder structure の上位ロジックを別 module に分離した。

## 今回実装したこと

### Folder Structure の定義を差し替えた

- `Folder Structure` は「library を選ぶ」「その library 直下のトップレベル folder を選ぶ」「その subtree 配下の song を tag 基準で album に再集約して grid 表示する」という挙動に変更した。
- `Artist` と `Album Artist` の意味や route はこの作業では変更しなかった。

### backend に folder structure 専用 command を追加した

- `get_music_folders` は library selector 専用の command として維持した。
- `get_folder_structure_roots` を追加し、library ごとの root node 一覧を返すようにした。
- `get_folder_structure_albums` を追加し、選択 node の subtree を走査して album card 用 DTO を返すようにした。

### root 取得で 2 系統のサーバー差を吸収した

- まず `getMusicDirectory(libraryId)` を試す。
- それが `API 70` または `Directory not found` のときだけ `getIndexes(musicFolderId=libraryId)` にフォールバックする。
- 戻り値には `source: directory | indexes` を含め、frontend が注記を出せるようにした。

### subtree 走査と album 集約を backend 側で行った

- subtree 走査は逐次 DFS に固定し、visited-id guard を入れた。
- `mediaType` が `song` の child だけを集約対象にした。
- album grouping は `albumId` を最優先にし、なければ `album + artist + year`、次に `album + artist`、最後に `Unknown Album` を使う形にした。
- album の表示 artist は song 群の `artist` の最頻値を採用した。
- cover art は group 内で最初に見つかった `coverArtId` を採用した。

### frontend の route と state を差し替えた

- route は `/browse/folders`、`/browse/folders/:libraryId`、`/browse/folders/:libraryId/:nodeId` の 3 段にした。
- `/browse/folders` は single-library のとき自動遷移し、multi-library のとき empty state を出すようにした。
- sidebar は `library selector + rootNodes` の 2 段構成にした。
- node ごとの album 集約結果と library ごとの rootNodes は session 内メモリ cache を持つようにした。

## 実装中に起きたこと

### commands.rs に責務が集まりすぎた

- 最初の実装では `commands.rs` に command 本体、Subsonic payload の parse/normalize、folder structure の fallback、subtree 走査、album 集約、unit test をまとめて置いてしまった。
- これは既存の分割基盤がなかったため一旦そこに載せた形だったが、結果として読む場所が多すぎて明らかに扱いづらかった。

### parser と domain logic を分けた

- `src-tauri/src/browse_parser.rs` に browse 系 payload の parse/normalize を切り出した。
- `src-tauri/src/folder_structure.rs` に folder structure の fallback、DFS、album 集約、関連 test を切り出した。
- `src-tauri/src/commands.rs` には Tauri command の入口、session から client を作る処理、cover art の binary fetch、API error の整形だけを残した。

## 所感

- `Folder Structure` のように subtree 走査や集約が必要な処理は frontend で持つより backend で持つほうが正しかった。
- 一方で backend-heavy にするほど command 層の責務整理は重要で、command の入口と domain logic を同じファイルに積むとすぐ読みづらくなることが分かった。
- コマンド層には「薄い adapter に近い command」と「複数 API 呼び出しや Rust 側の処理を含む command」の両方があるため、それらを同じ粒度で扱わないほうが整理しやすい。
- 今回の範囲では、`commands.rs` を入口に寄せ、複合処理を別 module に出した後のほうが構造は明確だった。

## 検証

- Rust unit test で root fallback、album 集約、Unknown Album fallback、artist 表示、visited-id guard を確認した。
- `cargo test`、`pnpm exec tsc --noEmit`、`pnpm exec vite build` を通した。
