<!--
author: Codex
date: 2026-03-24 12:05
version: 0.0.1
-->

## Subsonic API 呼び出し境界について

- Phase 1.5 の実装では、Subsonic / OpenSubsonic API への通信は Rust 側に一本化しました。
- UI からサーバーへ直接 fetch する経路は作らず、認証 query の生成、response envelope の解釈、API error の分類は Rust 側を正本として扱います。

## API key 認証で残しておく判断

- OpenSubsonic の API key 認証では、`apiKey` を使うときに `u` を送らない実装を採用しました。
- 接続時に API key 用の username 入力は受け取らず、必要な username は `tokenInfo` の結果を保存して使います。
- これは、公式ドキュメントの example URL ではなく、parameter table と extension の説明を優先して判断しました。example に `u` が含まれていても、それを実装根拠にはしません。

## 実装してみて残したい感触

- 認証方式ごとの差分を UI に持たせると、入力項目、保存値、接続確認、再接続の都合が分散しやすかったです。
- 逆に Rust 側で接続手順をまとめると、API key と password token の差分を一か所で扱えたので、仕様の食い違いに気づいたときの修正範囲が小さくなりました。
