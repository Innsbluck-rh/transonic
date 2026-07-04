<!--
author:   Codex
date:     2026-06-21 17:11
version:  0.0.1
-->

## 概要

- Windows の System Default 出力追従は、Transonic 独自の Windows default-device listener ではなく、CPAL 0.18 の default stream rerouting を利用する方針にしました。
- Windows の現行 runtime は WASAPI 出力であり、ASIO 出力は現行 runtime ではありません。
- `cpal` は 0.18.1 へ更新しました。

## 決定

- `outputDeviceId` が `None` / `null` の場合は、従来通り CPAL の default output device から stream を作成します。
- CPAL 0.18 の WASAPI default output stream は、Windows の既定出力変更時に stream を自動 reroute し、error callback に `ErrorKind::DeviceChanged` を通知します。
- `ErrorKind::DeviceChanged` は native error として扱いません。これは出力経路変更の通知であり、stream rebuild を要求する device lost とは区別します。
- `ErrorKind::Xrun` は recoverable、`ErrorKind::DeviceNotAvailable` と `ErrorKind::StreamInvalidated` は device lost として扱います。
- 明示的な `outputDeviceId` は従来通り WASAPI device を指定するものとして扱い、System Default 追従の対象ではありません。

## Platform 状況

- Windows は CPAL 0.18 の WASAPI default stream rerouting を利用します。
- macOS は CPAL 0.18 の CoreAudio default output stream に system default output 追従の実装があります。このADRの実装対象は Windows runtime です。
- Linux は CPAL 0.18 で PipeWire / PulseAudio backend が追加され、default host priority は PipeWire、PulseAudio、ALSA の順になりました。Windows と同等の default stream rerouting を Linux 全体の保証として扱う記述は確認していません。

## 依存関係

- `cpal` 0.18.1 は Windows で `windows` と `windows-core` を同時に使用します。
- Transonic では Tauri 側の依存と合わせるため、`windows` は 0.61 系、`windows-core` も 0.61 系に揃えています。
- `cpal` 0.18.1 の `asio` feature は `asio-sys` 0.3.0 を参照します。
- Transonic に残っているローカル `asio-sys` patch crate は 0.2.6 です。この patch crate は現行の WASAPI runtime 出力では使用しません。
