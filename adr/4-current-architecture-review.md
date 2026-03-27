<!--
author: Codex
date: 2026-03-27 22:10
version: 0.0.1
-->

## 現状アーキテクチャの見直しメモ

### 目的

- アルバム取得と UI 表示まわりの実装が一段落した段階で、現状のアーキテクチャ構成を見直す。
- 未実装機能の不足や、アーキテクチャと無関係な細部ではなく、構造上の問題を整理する。
- 特に以下を見る。
  - フロントエンドとバックエンドの責務分離
  - Tauri command と OpenSubsonic API 吸収層の分離
  - RPC 型定義の管理方法
  - セッション状態の同期方法
  - browse 系 UI を拡張する際のモデル整理

### 調査対象

- フロントエンド
  - `src/routes`
  - `src/features`
  - `src/models`
  - `src/stores`
  - `src/components`
- バックエンド
  - `src-tauri/src/commands.rs`
  - `src-tauri/src/session.rs`
  - `src-tauri/src/connection.rs`
  - `src-tauri/src/models.rs`
  - `src-tauri/src/profiles.rs`
  - `src-tauri/src/secrets.rs`
- OpenSubsonic 吸収層
  - `src-tauri/crates/opensubsonic-client`
- 参照した公式ドキュメント
  - OpenSubsonic API overview: https://opensubsonic.netlify.app/docs/opensubsonic-api/
  - getIndexes: https://opensubsonic.netlify.app/docs/endpoints/getindexes/
  - getMusicDirectory: https://opensubsonic.netlify.app/docs/endpoints/getmusicdirectory/
  - getMusicFolders: https://opensubsonic.netlify.app/docs/endpoints/getmusicfolders/
  - getOpenSubsonicExtensions: https://opensubsonic.netlify.app/docs/endpoints/getopensubsonicextensions/
  - tokenInfo: https://opensubsonic.netlify.app/docs/endpoints/tokeninfo/

### 先に結論

- 方向性そのものは大きく外していない。
  - 認証、セッション復元、接続判定、OpenSubsonic capability 判定を Rust 側へ寄せているのは正しい。
  - フロントエンドを「表示と軽い入力」に寄せようとしている方針も正しい。
- 一方で、現状は「責務の分離を始めた直後」の状態に近く、次の 4 点に歪みが見える。
  - RPC 型が Rust と TypeScript で二重管理されること
  - OpenSubsonic 差異吸収が command 層まで漏れていること
  - browse 系モデルの名前と実体がずれていること
  - セッション同期が request/response 前提で、backend 主導の更新を受けにくいこと

### 現状で良い点

- `SessionService` にプロフィール永続化とセッション復元がまとまっている。
- `ConnectionService` が接続確認と認証方式差異を担当しており、認証に関する知識がフロントへ漏れていない。
- `getOpenSubsonicExtensions` と `tokenInfo` を使って API key 認証可否と username 解決を Rust 側で処理している。
- `opensubsonic-client` を workspace crate として分離しているため、API 実装の独立性は確保しやすい。

### 指摘 1: RPC 型が二重管理になっている

#### 観察

- Rust 側の RPC DTO は `src-tauri/src/models.rs` にある。
- フロント側では `src/models/session.ts` と `src/models/bootstrap.ts` と `src/models/album.ts` と `src/models/folder.ts` に対応型を再定義している。
- 接続結果の型はさらに `src/routes/init_login.tsx` に置かれており、feature 層の `src/features/session/service.ts` から route の型を参照している。

#### 問題

- RPC 契約の単一ソースが存在しない。
- 型の追加や変更のたびに Rust と TypeScript を手で揃える必要がある。
- route が feature 層に型を提供しており、依存方向が逆転している。
- endpoint が増えるほど、どの型が正式定義なのか分かりにくくなる。

#### 所感

- いまの件数ではまだ耐えられるが、browse、playlists、queue、search、lyrics を同列に扱う構成として見ると保守コストが急に上がる。
- この調査では、構造上の問題として明示的に記録する価値があると判断した。

#### この時点の判断

- RPC 契約は Rust 側を基準に一元化されている方が整合しやすい。
- TypeScript 側の手書き再定義と route ローカル型は、契約の置き場所として不安定である。
- 型同期は手作業より生成に寄っている構成の方が、この時点の設計意図と噛み合っている。

### 指摘 2: OpenSubsonic 吸収層の責務が command 層まで漏れている

#### 観察

- `opensubsonic-client` には API ごとの request/response 型があるが、多くの response payload が `serde_json::Value` で返されている。
- そのため `src-tauri/src/commands.rs` が、Tauri command でありながら payload の個別 parse を多数持っている。
- cover art については binary request の組み立てまでは client 側、HTTP fetch と data URL 化は command 側が担当している。

#### 問題

- Tauri command 層が「アプリ外部 interface」と「OpenSubsonic 差異吸収」の両方を持っている。
- endpoint 追加のたびに `commands.rs` に parse 用 struct や変換関数が増えていく。
- API 仕様差異をどこで吸収するのかが曖昧になる。

#### 所感

- いまの `commands.rs` は command router というより adapter と parser を兼ねている。
- browse 系や playback/reporting 系まで視野に入れると、command 層が太り続ける構造である。

#### この時点の判断

- OpenSubsonic の仕様差異吸収は command 層ではなく、その下の client または adapter 層に寄っている方が責務が明確である。
- `commands.rs` が parser を兼ねる現状は、境界の厚みが大きい。
- command 層はフロント向け DTO の返却とセッション参照に寄っている方が理解しやすい。

### 指摘 3: browse 系モデルの命名と実体がずれている

#### 観察

- フロントでは `FolderIndexesResponse` / `FolderAlbumsResponse` / `FolderList` という名前を使っている。
- しかし実際に呼んでいるのは `getIndexes` と `getMusicDirectory` である。
- `getIndexes` は公式には「artist の index 構造」を返す API である。
- `getMusicDirectory` は「directory 配下の child 一覧」を返す API である。
- さらに top-level の music folder を扱う API として `getMusicFolders` が別にある。

#### 問題

- 現在 `Folder` と呼んでいるものの中に、実際には次の異なる概念が混ざっている。
  - top-level music folder
  - artist index
  - directory node
  - album として見せている directory child
- いまは 1 画面分なので問題が見えにくいが、browse モデルの意味が複数同居すると命名と実体のズレが表面化しやすい。

#### 所感

- `NavigationSideBar` にはすでに browse mode 切替の UI があり、単一の browse モデルに複数概念を押し込みやすい構造になっている。
- モデルを整理しないまま `Folder*` を広げると、名前の意味がさらに曖昧になる。

#### この時点の判断

- 内部モデル名は API の実体に対応している方が理解しやすい。
- `Folder` を artist index や directory child の総称として使うのは避けたい。
- この時点では `MusicFolder`、`ArtistIndex`、`DirectoryChild` のような粒度で捉える方が自然である。

### 指摘 4: セッション同期が request/response 前提になっている

#### 観察

- backend は `ActiveSessionState` を持っている。
- frontend は `bootstrap_app_state`、`connect_server_profile`、`delete_server_profile` の結果を受けて都度 `sessionStore` を更新している。
- 通常 API 呼び出し側では、セッションが壊れた場合でも単に文字列エラーを返すだけで、frontend の共通セッション状態へは自動反映されない。

#### 問題

- 状態同期が「その操作のレスポンスにセッション情報が含まれていること」に依存している。
- backend 主導の状態変化を受けにくい。
- 長寿命状態が増えるほど、画面ごとに整合性を保つ必要が出る。

#### 所感

- 現在の規模では問題が表面化していないが、backend-heavy 方針との相性はやや弱い。
- command の戻り値だけで state を同期する方式は、長寿命状態を扱う構成には向いていない。

#### この時点の判断

- session 同期を command の戻り値だけに依存する構成は、backend-heavy な境界として弱い。
- backend 主導の状態変化を受けられる interface を持つ方が自然である。
- bootstrap 用 snapshot と常時同期の経路は、役割が別である。

### 指摘 5: cover art の転送方式は構造上の負荷点として記録する

#### 観察

- cover art は backend で取得し、base64 data URL に変換して frontend に返している。
- frontend 側では Promise cache を持ち、画像をメモリ上で保持している。

#### 問題

- 画像一覧が増えるほど JSON 転送量、base64 膨張、JS heap 消費が増える。
- album grid や search result が大きくなると、最初に効いてくるのはこの経路の可能性が高い。

#### 所感

- 今の UI 規模では許容範囲だが、一覧件数が増える構成では負荷点になりやすい。

#### この時点の判断

- cover art を data URL で返す方式は、小規模一覧では成立するが恒久的な中核経路としては重い。
- 画像配信と JSON DTO を同じ経路で持つ必要はない。
- この調査では、画像転送は別の配信境界として扱う方が自然と判断した。

### 補足

- 今回の見直しでは、未実装機能の不足は評価対象にしていない。
- コメント量や見た目の細部など、アーキテクチャと関係のない指摘も対象外とした。
- 評価の中心は「現時点では成立しているが、構造上の整理不足が見える箇所」に置いた。
