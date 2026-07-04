<!--
author:   Codex
date:     2026-06-20 17:40
version:  0.0.1
-->

## 概要

- Windows の出力デバイス選択は WASAPI 限定の実装に戻しました。
- ASIO の runtime integration は、出力デバイス切り替えという目的に対して混入範囲とコード量が大きすぎたため、既存実装を継続せずスクラップしました。
- `windows-asio` Cargo feature、`asio-sys` patch crate、ASIO SDK / binary bundle / build support に関する材料は、runtime の出力デバイス切り替え実装ではありません。これらが残っていても、ASIO 出力が現行実装として存在することを意味しません。

## 決定

- `PlaybackCapabilities` は ASIO 対応可否を公開しません。Windows で公開する capability は `outputDeviceSelection` だけです。
- `PlaybackOutputDevice` は `id`、`name`、`isSystemDefault` だけを持ちます。`host: "wasapi" | "asio"` の分類は削除しました。
- Windows の出力デバイス列挙は CPAL の WASAPI host だけを対象にします。
- 設定値 `outputDeviceId` の `null` は Windows system default WASAPI を表します。
- 明示的な `outputDeviceId` は CPAL `DeviceId` として parse しますが、WASAPI 以外の host は出力対象として受け付けません。
- UI は `System Default (WASAPI)` と WASAPI device option だけを表示します。ASIO group、ASIO capability、ASIO の system default 例外処理は持ちません。
- ASIO 用の pause-silence、stop-silence、silence flush stream、release settle、ASIO host 列挙、ASIO device 解決、ASIO 向けテストは削除しました。

## 取り下げた方針

- ADR-18 に書かれた ASIO runtime integration は現行方針ではありません。
- ADR-18 の `PlaybackOutputDevice.host`、`PlaybackCapabilities.asioOutput`、ASIO host 列挙、ASIO device 解決、ASIO pause/stop/drop 特別処理、ASIO UI group、ASIO runtime test は実装指針として参照しません。
- ASIO を再度扱う場合、ADR-18 の実装を継ぎ足すのではなく、WASAPI の出力デバイス切り替えが成立している状態から別機能として再設計します。

## 残す知見

- ASIO driver は pause、stop、stream drop の前後で最後の有音 buffer を保持または反復することがあります。
- `cpal_stream.pause()` を先に呼ぶと、ASIO callback が止まり、無音 buffer を driver へ到達させる機会を失うことがあります。
- そのため、ASIO を作り直す場合は、pause 処理と callback 到達性を最初に確認します。過去の回避策は、stream を running のまま短時間だけ無音を書き、その後に pause/drop する形でした。
- この知見は ASIO 再考時の注意点であり、現行の WASAPI 出力デバイス切り替えには実装しません。
