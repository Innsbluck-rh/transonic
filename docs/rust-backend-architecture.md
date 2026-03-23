# Rust Backend Architecture

この資料は、現状の Rust バックエンドの全体像を把握するための引き継ぎ用メモです。

## 1. 何を担当しているか

現状の Rust バックエンドの中心責務は、音楽再生そのものではなく、以下の接続管理です。

- サーバープロフィールの作成・更新・削除
- 認証情報の安全な保存と復元
- 起動時の再接続
- 現在アクティブな接続先の管理
- 接続結果から得られるサーバー情報と capability 情報の保持

現時点では、バックエンドの中心は「Subsonic サーバーを利用可能な状態にすること」です。

## 2. モジュール構成

### 2.1 エントリポイント

- `src-tauri/src/lib.rs`
  - Tauri アプリを起動する
  - `ActiveSessionState` をアプリ全体に登録する
  - Tauri command を公開する

- `src-tauri/src/main.rs`
  - `transonic_lib::run()` を呼ぶだけの薄い起動用ファイル

### 2.2 Tauri command 境界

- `src-tauri/src/commands.rs`
  - フロントエンドから呼ばれる command を定義する
  - 各 command は `SessionService` に処理を委譲する
  - command 自体は薄く、入力受け取りとサービス生成が主な役割

公開されている command は次の 5 つです。

- `bootstrap_app_state`
- `connect_server_profile`
- `activate_server_profile`
- `disconnect_active_profile`
- `delete_server_profile`

### 2.3 アプリケーションサービス

- `src-tauri/src/session.rs`
  - バックエンドの中心
  - プロフィール、secret、Subsonic API クライアントを束ねる
  - 接続成功/失敗に応じて、保存状態とメモリ状態を更新する

`SessionService` は以下の依存を持ちます。

- `ProfilesFile` に対する読み書き
- `SecretStore`
- `SubsonicApi`

### 2.4 プロフィール保存

- `src-tauri/src/profiles.rs`
  - `profiles-v1.json` の読み書きを担当する
  - 接続先プロフィールのメタデータを保持する

保存されるのは主に次の情報です。

- `profile_id`
- 表示名
- 正規化済みサーバー URL
- 認証方式
- username
- 最後の接続状態
- 最後に取得した capability 情報
- 最後に見えたサーバー情報
- 最終接続時刻
- 最後にアクティブだったかどうか

### 2.5 秘密情報保存

- `src-tauri/src/secrets.rs`
  - password や API key を OS キーリングに保存する
  - ファイルには secret を保存しない

設計上の分離は次の通りです。

- JSON ファイル: プロフィールの台帳
- OS キーリング: secret 本体

### 2.6 Subsonic 通信

- `src-tauri/src/subsonic_client.rs`
  - HTTP 通信を担当する
  - URL 正規化
  - `ping.view` による接続確認
  - `getOpenSubsonicExtensions.view` による capability 取得
  - `tokenInfo.view` による API key 利用時の username 解決
  - Subsonic エラーの分類

### 2.7 データ契約

- `src-tauri/src/models.rs`
  - command の入出力
  - 認証入力
  - active session
  - profile summary
  - capability 情報
  - 接続結果 enum

## 3. 状態の持ち方

状態は 3 層に分かれています。

### 3.1 メモリ上の現在値

- `ActiveSessionState = Mutex<Option<ActiveSession>>`
- 実行中のアプリが「いま接続中と見なしているもの」
- アプリ終了で消える

### 3.2 永続化されたプロフィール台帳

- `profiles-v1.json`
- 再接続に必要な基本情報を持つ
- secret そのものは含まない

### 3.3 永続化された secret

- OS キーリング
- `service_name + profile_id` で保存される
- password / API key を保持する

## 4. 中核データモデル

### 4.1 `AuthInput`

認証入力は次の 2 種です。

- `Password`
- `ApiKey`

どちらも username を持ちます。

### 4.2 `ActiveSession`

現在アクティブな接続先を表します。

含まれるもの:

- `profile_id`
- 正規化済みサーバー URL
- username
- 認証方式
- API version
- server type / server version
- capability matrix

含まれないもの:

- password
- API key

### 4.3 `SavedProfileSummary`

UI が一覧表示に使う軽量なプロフィール情報です。

### 4.4 `CapabilityMatrix`

OpenSubsonic 拡張の有無を、既知のフラグに展開したものです。

主なフラグ:

- `api_key_auth`
- `index_based_queue`
- `playback_report`
- `transcoding`
- `transcode_offset`
- `song_lyrics`

## 5. 起動時の流れ

`bootstrap_app_state` の流れは次の通りです。

1. `profiles-v1.json` を読む
2. すでにメモリ上に active session があればそれを返す
3. そうでなければ `is_last_active` なプロフィールを探す
4. キーリングから secret を復元する
5. Subsonic サーバーへ再接続を試みる
6. 成功時は active session を復元し、プロフィール状態も更新する
7. 失敗時は `Offline` または `ReauthRequired` として返す

復元結果は `RestoreStatus` で表されます。

- `None`
- `Restored`
- `Offline`
- `ReauthRequired`

## 6. 接続時の流れ

`connect_server_profile` は新規作成と既存更新の両方を担当します。

### 6.1 成功時

1. サーバー URL と認証情報で接続確認する
2. `profile_id` を確定する
3. プロフィール情報を upsert する
4. secret をキーリングへ保存する
5. active session を更新する
6. `Connected` を返す

### 6.2 失敗時

- 新規プロフィール作成中なら、基本的に保存しない
- 既存プロフィール更新中なら、`last_connection_state` を更新する
- エラー結果を `ConnectServerProfileResult` として返す

## 7. 既存プロフィールの再アクティブ化

`activate_server_profile` の流れ:

1. プロフィール台帳から対象プロフィールを探す
2. キーリングから secret を読む
3. 接続確認を行う
4. 成功なら active session を切り替える
5. 失敗ならプロフィール状態を更新し、active session を外す

## 8. 切断と削除

### 8.1 `disconnect_active_profile`

この処理は「サーバーとの明示的 logout」ではありません。

やっていること:

- `is_last_active` を外す
- メモリ上の active session を `None` にする

### 8.2 `delete_server_profile`

やっていること:

- プロフィール台帳から削除する
- キーリングから secret を削除する
- 必要なら active session も外す

## 9. 認証方式の扱い

### 9.1 Password 認証

平文パスワード送信ではなく、Subsonic の token 認証を使います。

送るもの:

- `u`: username
- `t`: `md5(password + salt)`
- `s`: salt

### 9.2 API key 認証

`apiKey` クエリを使います。

ただし接続前に、サーバーが OpenSubsonic 拡張として `apiKeyAuthentication` を広告しているか確認します。

広告していない場合:

- 接続は試みず `UnsupportedAuth` を返す

## 10. URL の正規化

接続時にサーバー URL は正規化されます。

主なルール:

- `http://` または `https://` が必須
- host が必須
- 埋め込み username / password は削除
- query と fragment は削除
- パス末尾を `/rest` にそろえる

例:

- `https://demo.example` -> `https://demo.example/rest`
- `https://demo.example/subsonic/` -> `https://demo.example/subsonic/rest`

## 11. エラーの見方

通信層では `ConnectFailure` として次を区別します。

- `Auth`
- `Network`
- `Server`
- `UnsupportedAuth`

ただしプロフィール状態としては、より粗い `LastConnectionState` に圧縮されます。

- `Never`
- `Ok`
- `Offline`
- `ReauthRequired`

対応関係:

- `Auth` / `UnsupportedAuth` -> `ReauthRequired`
- `Network` / `Server` -> `Offline`

## 12. この実装でまだ中心ではないもの

現時点では、次の領域はバックエンドの主責務にまだ入っていません。

- 楽曲一覧取得
- アルバム/アーティスト参照
- ストリーミング URL の取得
- 再生キュー管理
- scrobble / playback report 送信
- 歌詞取得

つまり、保存されている `CapabilityMatrix` は将来の分岐の準備としては意味がありますが、現時点では主に「接続時に取得して保持している情報」です。

## 13. 一言でいうと

現状の Rust バックエンドは、以下の 3 つをまとめる接続管理サブシステムです。

- 接続先プロフィール管理
- 認証情報管理
- 接続状態とサーバー能力の確立

音楽プレイヤーの本体というより、「どの Subsonic 系サーバーに、どの認証方式で、現在接続できているか」を管理する基盤層として理解するのが最も正確です。
