<!--
author: Codex
date: 2026-04-01 15:47
version: 0.0.1
-->

## 概要

- Android 向けのビルドで `src-tauri/src/secrets.rs` が `keyring` crate を直接参照しており、`Cargo.toml` に Android 向け `keyring` 依存がないためコンパイルが止まっていました。
- 今回は Android での動作確認を優先し、Android だけ app config 配下の JSON ファイルへ秘密情報を保存する暫定実装を採用しました。
- Windows、macOS、Linux では従来どおり OS keyring を使う構成を維持しました。

## 今回決めたこと

### Android の secret store は file-backed fallback を使う

- Android では `AppSecretStore` を file-backed な実装に切り替え、`secrets-v1.json` に資格情報を保存する構成にしました。
- 保存先は Tauri の app config directory 配下とし、profile metadata と同じく app-private な設定領域に寄せました。
- secret の保存、読込、削除のインターフェースは既存の `SecretStore` trait をそのまま使い、session の上位ロジックは変更しない構成にしました。

### desktop の keyring 実装は維持する

- Windows、macOS、Linux では `keyring::Entry` を使う既存実装を維持しました。
- Android 対応のために desktop 側の依存や保存仕様を変えない判断を取りました。

### secret store の生成は呼び出し側で分岐を隠す

- `SessionService` に OS 分岐を持ち込まず、`commands/common.rs` で `create_secret_store(&config_dir)` を呼び出して注入する構成にしました。
- これにより、session の接続・復元・削除ロジックは保存先の違いを意識しないまま維持できます。

## 判断理由

### Android では secure store の完成度よりビルド互換を優先した

- 現時点では Android 上で画面や再生まわりを確認できる状態が必要であり、まず Rust 側が Android ターゲットで `keyring` 参照により止まらないことが重要でした。
- そのため、今回は secure store の完成度より、session 保存と復元の経路が Android でも成立することを優先しました。

### `SecretStore` 境界がすでにあったため差し替えコストが低かった

- 既存実装は `SessionService<S, A>` と `SecretStore` trait に分離されており、保存先の差し替えを trait 実装の追加だけで扱える状態でした。
- この構造を利用したほうが、Android の事情を session や command の主要ロジックへ広げずに済みます。

### desktop と Android の事情を同じ実装で無理に揃えないほうが影響範囲を抑えられた

- Android だけ compile failure を起こしていたため、問題のある target だけを別実装に切り出すほうが影響範囲を限定できます。
- desktop 側の keyring 挙動まで同時に変えると、今回の目的である Android 着手と無関係な差分が増えるため避けました。

## 今回受け入れた制約

- Android では OS の secure storage ではなく、app-private 領域のファイル保存を使っています。
- そのため、desktop の keyring と同等の保護水準を持つ構成ではありません。
- この ADR で記録する意思決定は、Android でのビルド互換と session 復元経路の維持を優先した、暫定の保存方式を採用したことです。
