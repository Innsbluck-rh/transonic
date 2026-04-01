<!--
author: Codex
date: 2026-04-01 17:27
version: 0.0.1
-->

## 概要

- Android の再生方式の選定にあたり、Tauri plugin は使わないことに決定しました。
- Android 向けの再生 backend として Media3/ExoPlayer を採用し、MediaPlayer は採用しないことに決定しました。
- この選定は、再生状態の正本を Rust backend に置き、OS 依存の再生処理を shim 境界の内側に閉じ込める既存方針を前提に行いました。

## 今回決めたこと

### Android 再生では Tauri plugin を採用しない

- Android の再生方式は、Tauri 用の外部 plugin に依存しない方針にしました。
- 候補として見ていた `tauri-plugin-native-audio` は採用しないことに決定しました。

### Android の再生 backend は Media3/ExoPlayer を採用する

- Android 向けの再生 backend として Media3/ExoPlayer を採用することに決定しました。
- Android 向けの再生 backend として MediaPlayer は採用しないことに決定しました。

### Android の方式選定は既存の再生境界を前提に行う

- 再生状態の正本は Rust backend に置く方針を維持します。
- OS 依存の再生処理は shim 境界の内側に閉じ込める方針を維持します。
- Windows 再生 backend にある音源取得と再生処理の詳細が上位層へ広がっていないことを、今回の選定の前提として確認しました。

## 判断理由

### Android の主要な再生責務を外部 plugin に置かないほうが判断と保守の主体を保てる

- このプロジェクトでは、再生状態と制御の正本を Rust backend に置く構成をすでに採用しています。
- Android の主要な再生責務を Tauri plugin へ委ねると、再生まわりの判断と追跡の主体が repository の外へ出やすくなります。
- そのため、Android の再生方式は project 内で直接扱える構成を優先しました。

### Media3/ExoPlayer のほうが Android の再生基盤として判断しやすい

- Media3/ExoPlayer は Android の現行の media 基盤として位置づけられており、新規の再生実装に対する判断材料が揃っています。
- MediaPlayer は単純な再生には使えますが、再生状態、buffering、media session、streaming protocol まわりの判断を進める基盤としては比較対象に劣りました。
- そのため、Android 向けの再生 backend は Media3/ExoPlayer を採用する判断にしました。

### 既存の shim 境界を崩さずに Android の方式を選定できる

- 現在の再生構造では、queue や再生状態の管理は controller にあり、OS 依存の再生処理は backend shim の内側に分離されています。
- そのため、Android の方式選定も上位層の責務を変えずに判断できる状態でした。

## 参照

- Android Developers, Media3 ExoPlayer: https://developer.android.com/media/media3/exoplayer
- Android Developers, MediaPlayer: https://developer.android.com/reference/android/media/MediaPlayer
- Android Developers, MediaSessionService: https://developer.android.com/media/media3/session/background-playback
- Android Developers, Supported formats in ExoPlayer: https://developer.android.com/media/media3/exoplayer/supported-formats
- OpenSubsonic stream endpoint: https://opensubsonic.netlify.app/docs/endpoints/stream/
- tauri-plugin-native-audio: https://github.com/uvarov-frontend/tauri-plugin-native-audio
