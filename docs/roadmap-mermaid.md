# 実装フロー図

このファイルは、2026-03-23 時点の実装状態に基づく現状フローと、今後の実装を進めるための Phase フローを Mermaid でまとめたものです。

## 1. 現状のフロー

```mermaid
flowchart LR
  UI["Solid UI<br/>src/App.tsx<br/>- invoke(bootstrap_app_state)<br/>- invoke(connect_server_profile)<br/>- invoke(activate_server_profile)"] --> CMD["Tauri commands<br/>src-tauri/src/commands.rs"]
  CMD --> SESSION["SessionService<br/>src-tauri/src/session.rs<br/>- profile 読み書き<br/>- active session 更新<br/>- secret store 利用"]
  SESSION --> CONNECTION["ConnectionService<br/>src-tauri/src/connection.rs<br/>- normalize_base_url<br/>- ServerProbe で capability 確認<br/>- OpenSubsonicClient で接続確認"]
  CONNECTION --> CLIENT["OpenSubsonicClient<br/>src-tauri/crates/opensubsonic-client<br/>- config / auth / envelope / error<br/>- system / browsing / lists / search<br/>- playlists / retrieval / annotation"]
  CLIENT --> SERVER["Subsonic / OpenSubsonic Server"]
  SERVER --> CLIENT
  SESSION --> STORE["永続化層<br/>- profiles-v1.json<br/>- OS keyring"]
  CLIENT --> RESULT{"AppBootstrap / ConnectServerProfileResult"}
  STORE --> SESSION
  RESULT -->|active_session あり| HOME["Home 相当の画面<br/>- active session 表示<br/>- profile 一覧表示"]
  RESULT -->|active_session なし| LOGIN["Login / Profile 選択画面"]

  HOME -. 未実装 .-> TODO["未実装の主要機能<br/>- browse / album / search<br/>- queue domain<br/>- playback shim<br/>- reportPlayback / scrobble"]

  classDef warn fill:#fff2cc,stroke:#b38f00,color:#222;
  class TODO warn;
```

## 2. 完成イメージの責務分離

```mermaid
flowchart LR
  UI["Frontend UI<br/>- 描画<br/>- 入力受付<br/>- Rust state の表示"] --> CORE["Rust app core<br/>- session / auth<br/>- capability matrix<br/>- library access<br/>- queue state<br/>- playback control"]
  CORE --> API["Subsonic / OpenSubsonic API"]
  CORE --> SHIM["OS playback shim<br/>初期実装は Windows のみ"]
  SHIM --> AUDIO["ネイティブ再生"]
  SHIM --> CORE

  NOTE["設計方針<br/>UIは薄く保つ<br/>Rustを正本にする<br/>Shimは再生だけ担当"]:::note
  NOTE -. 補足 .-> CORE

  classDef note fill:#e9f4ff,stroke:#4a7eb8,color:#222;
```

## 3. Phase で進める提案フロー

```mermaid
flowchart TB
  NOW["現在地点<br/>Phase 1 相当まで実装済み<br/>- session / profile<br/>- ping<br/>- getOpenSubsonicExtensions<br/>- token / API key"] --> P15

  subgraph P1["Phase 1: Session / Capability 基盤"]
    P1A["Rust<br/>AuthMethod 抽象化<br/>- token(password)<br/>- API key"]
    P1B["Rust<br/>ServerProfile / CapabilityMatrix<br/>- ping<br/>- getOpenSubsonicExtensions"]
    P1C["UI<br/>ログイン画面は薄いまま"]
    P1A --> P1B --> P1C
  end

  P1 --> P15

  subgraph P15["Phase 1.5: OpenSubsonic Client 基盤再編"]
    P15A["Rust<br/>low-level client 再設計<br/>- config / auth / envelope / error"]
    P15B["Rust<br/>カテゴリ別 API module<br/>- system / browsing / lists / search<br/>- playlists / retrieval / annotation"]
    P15C["Rust<br/>JSON / binary 分離<br/>- stream / download / coverArt<br/>- getTranscodeStream"]
    P15A --> P15B --> P15C
  end

  P15 --> P2

  subgraph P2["Phase 2: 閲覧 / キュー"]
    P2A["Rust<br/>app service 実装<br/>- home / browse / album 向け read-only 利用<br/>- getMusicFolders / getArtists / getAlbum<br/>- search3 / getAlbumList2"]
    P2B["Rust<br/>Queue domain<br/>- getPlayQueue / savePlayQueue<br/>- indexBasedQueue があれば利用"]
    P2C["UI<br/>home / browse / album / queue"]
    P2A --> P2B --> P2C
  end

  P2 --> P3

  subgraph P3["Phase 3: Windows 再生 v1"]
    P3A["Rust<br/>PlayerBackend interface<br/>- load / play / pause / seek / stop / volume"]
    P3B["Windows shim<br/>in-process か sidecar で実装"]
    P3C["Rust<br/>PlaybackController<br/>- stream URL 解決<br/>- state/event 正規化"]
    P3A --> P3B --> P3C
  end

  P3 --> P4

  subgraph P4["Phase 4: 再生同期 / 報告 / 安定化"]
    P4A["OpenSubsonic 対応サーバー<br/>reportPlayback"]
    P4B["非対応サーバー<br/>scrobble にフォールバック"]
    P4C["Rust/UI<br/>- queue 復元<br/>- 再接続時の整合<br/>- now playing 表示"]
    P4A --> P4C
    P4B --> P4C
  end

  P4 --> GOAL["到達点<br/>UI = ガワ<br/>Rust = 状態と制御の正本<br/>Shim = 再生エンジン"]
```

## 4. Phase 1.5 の補足

- Phase 1.5 は、UI 機能追加の前に OpenSubsonic の低レベル通信レイヤーを作り直すためのフェーズです。
- このフェーズでは `docs/OpenSubSonic/docs` 配下の endpoint を、カテゴリ別の low-level wrapper として先に実装してよいです。
- ただし「low-level wrapper の実装完了」と「画面でその API を使う app service の実装」は分けて考えます。
- そのため Phase 2 の `read-only API 実装` は、全 endpoint の Rust 化そのものではなく、画面に必要な読み取り系 API を使う上位機能の実装を指すものとして整理します。
