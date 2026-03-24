# OpenSubsonic Client Design

この文書は、Rust バックエンド内に実装する OpenSubsonic 通信レイヤーの設計書です。

対象は「OpenSubsonic / Subsonic の低レベル API ラッパ」に限定します。
アプリ固有の状態管理、再生キュー制御、画面向け DTO 変換、複数 API の統合処理は対象外です。

## 1. 目的

本レイヤーの目的は次の通りです。

- OpenSubsonic / Subsonic の endpoint を Rust から一貫した方法で呼び出せるようにする
- 認証、共通 query、response envelope、エラー解釈を共通化する
- JSON endpoint と binary endpoint を同じ client 配下で整理して扱えるようにする
- 上位層が profile 管理や UI 都合を持ち込まずに利用できる形にする
- 今後 endpoint 数が増えても、ファイル分割と型分割が破綻しない構造にする

## 2. 非目的

本レイヤーでは次のことは行わない。

- active session の保持
- profile の保存
- secret の保存
- 起動時の再接続フロー
- 画面単位の複合ユースケース
- キャッシュ戦略
- プレイヤー内部状態との同期
- 複数サーバーの統合表示

これらは上位の application service が担当する。

## 3. 現状実装に対する判断

現状の [`src-tauri/src/subsonic_client.rs`](C:/Users/n4505/Documents/transonic/src-tauri/src/subsonic_client.rs) は、
ログイン確認までの実装としては十分だが、低レベル通信層の本体としては拡張しない方がよい。

主な理由:

- `connect()` が低レベル通信とアプリ固有の接続判定を同時に担っている
- エラー型が UI 側の都合に寄っている
- response envelope、認証、capability、接続確認が 1 ファイルに密集している
- JSON API と binary API の違いを吸収する構造になっていない
- endpoint 数が増えた時に trait も file も肥大化しやすい

方針:

- `session.rs` 側の接続管理という考え方は残す
- 低レベル client は作り直す
- URL 正規化、token 認証、OpenSubsonic extension 取得の知見は流用する

## 4. 基本方針

### 4.1 1 client = 1 server + 1 auth

低レベル client は、生成時点で接続先サーバーと認証方式が確定しているものとして扱う。

この方針により:

- 各 method の引数から `server_url` と `auth` を消せる
- request 型が endpoint 固有の引数だけに集中する
- 上位層が「どの接続先を使うか」を管理し、下位層は「その接続先に何を投げるか」に専念できる

### 4.2 low-level と app-level を分離する

low-level client は次だけを行う。

- HTTP request を作る
- 共通 query を付ける
- JSON / binary を受け取る
- OpenSubsonic / Subsonic の envelope を解釈する
- API error を Rust error に変換する

low-level client は次を行わない。

- `Connected` / `AuthError` のような UI 向け結果 enum の生成
- `LastConnectionState` への丸め込み
- profile 更新
- secret 更新

### 4.3 OpenSubsonic を第一級に扱う

OpenSubsonic の拡張 endpoint は後付けの例外ではなく、正式なカテゴリとして扱う。

ただし構造上は:

- Subsonic 共通 endpoint
- OpenSubsonic 拡張 endpoint

を同一 client 内の別 module として管理する。

## 5. 推奨構成

可能なら別 crate に分離する。

推奨:

- `src-tauri/crates/opensubsonic-client`

難しければ、まずは `src-tauri/src/opensubsonic/` を作る。

推奨ディレクトリ構成:

```text
opensubsonic/
  mod.rs
  client.rs
  config.rs
  auth.rs
  error.rs
  transport.rs
  envelope.rs
  extensions.rs
  api/
    mod.rs
    system.rs
    browsing.rs
    lists.rs
    search.rs
    playlists.rs
    retrieval.rs
    annotation.rs
    sharing.rs
    podcast.rs
    jukebox.rs
    radio.rs
    chat.rs
    users.rs
    bookmarks.rs
    scanning.rs
    transcoding.rs
  types/
    mod.rs
    common.rs
    system.rs
    browsing.rs
    lists.rs
    search.rs
    playlists.rs
    retrieval.rs
    annotation.rs
    sharing.rs
    podcast.rs
    jukebox.rs
    radio.rs
    chat.rs
    users.rs
    bookmarks.rs
    scanning.rs
    transcoding.rs
```

この分割は `docs/OpenSubSonic/docs/Endpoints` のカテゴリと対応が取りやすい。

## 6. 公開 API の考え方

巨大な 1 trait に全 endpoint を載せる設計は採用しない。

理由:

- 80 以上の method を 1 trait に載せると mock が重くなる
- 上位層の依存が不必要に広がる
- 一部カテゴリだけ使いたい時にも全体依存になる

採用方針:

- concrete client `OpenSubsonicClient` を中心に置く
- カテゴリごとに trait を分ける
- 実装は同じ client に対する trait impl とする

例:

```rust
pub struct OpenSubsonicClient {
    config: ClientConfig,
    transport: ReqwestTransport,
}

#[async_trait]
pub trait SystemApi {
    async fn ping(&self) -> Result<PingResponse, ApiError>;
    async fn get_open_subsonic_extensions(&self) -> Result<ExtensionsResponse, ApiError>;
    async fn token_info(&self) -> Result<TokenInfoResponse, ApiError>;
}

#[async_trait]
pub trait BrowsingApi {
    async fn get_artists(&self, req: GetArtistsRequest) -> Result<GetArtistsResponse, ApiError>;
    async fn get_artist(&self, req: GetArtistRequest) -> Result<GetArtistResponse, ApiError>;
    async fn get_album(&self, req: GetAlbumRequest) -> Result<GetAlbumResponse, ApiError>;
}
```

これにより、上位層は必要なカテゴリだけに依存できる。

## 7. config と認証

### 7.1 ClientConfig

```rust
pub struct ClientConfig {
    pub base_url: Url,
    pub auth: Auth,
    pub client_name: String,
    pub api_version: ApiVersion,
}
```

要件:

- `base_url` は `/rest` まで正規化済み
- `client_name` は Tauri app 名から受け取れるようにする
- `api_version` は文字列ではなく薄い型を持たせてもよい

### 7.2 Auth

```rust
pub enum Auth {
    Token {
        username: String,
        password: SecretString,
    },
    ApiKey {
        api_key: SecretString,
    },
    LegacyPassword {
        username: String,
        password: SecretString,
    },
}
```

設計意図:

- API key 認証時に username を必須にしない
- token 認証を通常経路として扱う
- legacy password は互換用途として隔離する

## 8. transport 層

transport は `reqwest` を直接隠す薄い層とする。

責務:

- 共通 query の付与
- GET / POST form の送信
- timeout の適用
- response body の取得

少なくとも次の 2 系統を持つ。

- `send_json`
- `send_binary`

例:

```rust
pub struct ReqwestTransport {
    json_client: reqwest::Client,
    streaming_client: reqwest::Client,
}
```

理由:

- metadata API と stream API では timeout 要件が異なる
- `stream`, `download`, `getCoverArt`, `getAvatar`, `getTranscodeStream` は JSON と別扱いが必要

## 9. envelope と response 解釈

ほぼすべての JSON endpoint は `subsonic-response` を返す。

したがって low-level 層で共通 envelope を定義する。

```rust
pub struct Envelope<T> {
    pub meta: ResponseMeta,
    pub payload: T,
}

pub struct ResponseMeta {
    pub status: ResponseStatus,
    pub api_version: String,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub open_subsonic: Option<bool>,
}
```

注意点:

- Subsonic では `type`, `serverVersion`, `openSubsonic` がないことがある
- OpenSubsonic では追加 field がある
- low-level 層では欠損を `Option` で受ける

失敗時は `error` object を `ApiError::Api` へ落とす。

## 10. error 設計

`ConnectFailure` のような接続専用 error ではなく、全 endpoint 共通の `ApiError` を持つ。

```rust
pub enum ApiError {
    InvalidUrl(String),
    Transport(reqwest::Error),
    HttpStatus {
        status: StatusCode,
        body_preview: Option<String>,
    },
    Decode {
        message: String,
        body_preview: Option<String>,
    },
    Api {
        code: u32,
        message: Option<String>,
        help_url: Option<String>,
        meta: ResponseMeta,
    },
    UnsupportedExtension {
        extension: ExtensionName,
    },
    Protocol(String),
}
```

方針:

- low-level 層では `Offline`, `ReauthRequired` などの app 用分類はしない
- OpenSubsonic の `helpUrl` は保持する
- HTTP error と API error を分ける
- decode error では body preview を少量だけ保持できるようにする

## 11. endpoint ごとの request / response 型

### 11.1 request 型

request 型は endpoint 単位で作る。

例:

```rust
pub struct GetAlbumList2Request {
    pub list_type: AlbumListType,
    pub size: Option<u32>,
    pub offset: Option<u32>,
    pub from_year: Option<u32>,
    pub to_year: Option<u32>,
    pub genre: Option<String>,
    pub music_folder_id: Option<String>,
}
```

原則:

- `Option` を使って仕様上の任意項目を表す
- enum にできる文字列は enum にする
- query 名と Rust field 名は分けてよい

### 11.2 response 型

response 型は docs の response 名に寄せる。

例:

- `AlbumList2`
- `AlbumId3`
- `ArtistWithAlbumsId3`
- `PlayQueueByIndex`

原則:

- OpenSubsonic docs の `Responses/*` を基準にする
- 実サーバー差異で欠損しうる field は `Option` を使う
- アプリ都合の整形はしない

### 11.3 version 違い endpoint の扱い

`getAlbumList` と `getAlbumList2` のような version 違い endpoint は無理に統合しない。

理由:

- 返却構造が異なる
- 後で差異が増えても壊れにくい

共通化するなら:

- enum
- 小さな共有型
- helper 関数

に限る。

## 12. OpenSubsonic extension の扱い

extension 対応は low-level client から切り離さず、明示的に表現する。

```rust
pub struct ServerFeatures {
    pub open_subsonic: bool,
    pub extensions: BTreeMap<ExtensionName, Vec<u32>>,
}
```

用途:

- `apiKeyAuthentication`
- `playbackReport`
- `indexBasedQueue`
- `transcoding`
- `songLyrics`

方針:

- capability 取得 endpoint 自体は system API に置く
- 他の extension endpoint は通常 method として実装する
- 事前 capability check を low-level で強制するかどうかは method ごとに決める

推奨:

- low-level は原則送信可能にする
- 上位層が capability を見て利用可否を判断する

例外:

- `tokenInfo` のように OpenSubsonic 前提が強い API は `UnsupportedExtension` を返してもよい

## 13. JSON API と binary API

binary endpoint は JSON endpoint と同じ戻り値にしない。

対象例:

- `stream`
- `download`
- `hls`
- `getCoverArt`
- `getAvatar`
- `getTranscodeStream`
- `downloadPodcastEpisode`

推奨戻り値:

```rust
pub struct BinaryResponse {
    pub response: reqwest::Response,
}
```

または:

```rust
pub struct PreparedBinaryRequest {
    pub url: Url,
    pub headers: HeaderMap,
}
```

選択基準:

- Rust 側で直接取得して Tauri へ流すなら `BinaryResponse`
- フロントや proxy に処理を委ねるなら `PreparedBinaryRequest`

## 14. 現行 session/profile 層との関係

low-level client 導入後の責務分担:

- `opensubsonic/*`
  - 純粋な API 通信
- `session.rs`
  - 接続確認のオーケストレーション
  - active session 更新
  - profile / secret の保存
- `commands.rs`
  - Tauri command 境界

つまり `connect()` 相当の処理は low-level client に置かず、
上位の connection service が以下を組み合わせて作る。

- `normalize_base_url`
- `ping`
- `get_open_subsonic_extensions`
- `token_info`

## 15. 具体的な module 対応案

`docs/OpenSubSonic/docs/Endpoints` を基準に、まず次の順で実装するのがよい。

### Phase 1: 接続と基盤

- `system.rs`
  - `ping`
  - `getOpenSubsonicExtensions`
  - `tokenInfo`
  - `getLicense`

### Phase 2: 閲覧系

- `browsing.rs`
  - `getMusicFolders`
  - `getGenres`
  - `getArtists`
  - `getArtist`
  - `getAlbum`
  - `getSong`

- `lists.rs`
  - `getAlbumList`
  - `getAlbumList2`
  - `getRandomSongs`
  - `getStarred`
  - `getStarred2`

- `search.rs`
  - `search3`
  - 必要なら `search2`, `search`

### Phase 3: 再生周辺

- `retrieval.rs`
  - `stream`
  - `download`
  - `getCoverArt`
  - `getCaptions`
  - `getLyrics`
  - `getLyricsBySongId`

- `annotation.rs`
  - `star`
  - `unstar`
  - `setRating`
  - `scrobble`
  - `reportPlayback`

### Phase 4: 補助機能

- `playlists.rs`
- `bookmarks.rs`
- `sharing.rs`
- `podcast.rs`
- `users.rs`
- `radio.rs`
- `chat.rs`
- `scanning.rs`
- `transcoding.rs`

## 16. テスト方針

### 16.1 単体テスト

対象:

- URL 正規化
- token 生成
- query 生成
- enum 変換
- envelope 解釈
- error code 解釈

### 16.2 HTTP モックテスト

対象:

- endpoint ごとの query 送信
- JSON decode
- OpenSubsonic field の有無
- error response の decode

### 16.3 実サーバー確認

少なくとも次は実サーバーで確認した方がよい。

- OpenSubsonic 非対応サーバー
- OpenSubsonic 対応サーバー
- API key 対応サーバー
- binary endpoint

理由:

- docs は強い参照になるが、実装差異や欠損 field がありうるため

## 17. 移行計画

### Step 1

新しい `opensubsonic/` module を追加する。

### Step 2

`system` カテゴリだけを新構造で実装する。

対象:

- `ping`
- `getOpenSubsonicExtensions`
- `tokenInfo`

### Step 3

既存の `SessionService` の接続確認処理を新 client に差し替える。

### Step 4

`subsonic_client.rs` に残る旧実装を削る。

### Step 5

閲覧系 endpoint をカテゴリ単位で順次追加する。

## 18. 設計上の判断まとめ

- low-level client は作り直す
- session/profile/secret 管理は上位層に残す
- client は 1 server + 1 auth に束縛する
- endpoint はカテゴリで分割する
- request/response は endpoint 単位で型を作る
- OpenSubsonic 拡張は正式な module として扱う
- JSON と binary は最初から別の経路で扱う
- UI 向けの結果分類は low-level 層に持ち込まない

## 19. 最終的なイメージ

最終構成は次の 3 層になる。

1. OpenSubsonic low-level client
   - endpoint を素直に呼ぶ層
2. session / connection service
   - 接続と永続状態を管理する層
3. Tauri command / frontend
   - UI から利用する層

この形にしておくと、今後 API を増やす作業は
「low-level に endpoint を追加する」
という単純作業に近づき、
ログイン管理やプレイヤー機能の実装と衝突しにくくなる。
