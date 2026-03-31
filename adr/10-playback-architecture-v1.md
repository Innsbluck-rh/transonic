<!--
author: Codex
date: 2026-03-31 16:52
version: 0.0.1
-->

## 概要

- 再生実装の正本は Rust backend に置くことに決定した。
- queue は順序を持つ配列として扱い、現在位置は曲 ID ではなく queue 上の index で扱うことに決定した。
- OS 依存の再生処理は shim 境界の内側に閉じ込め、v1 では Windows 実装のみを持つことに決定した。
- server への再生報告は再生成立の前提にはせず、OpenSubsonic/Subsonic の capability に応じて使い分けることに決定した。

## 今回決めたこと

### 再生状態の正本は Rust backend に置く

- play、pause、seek、stop、next、prev、queue 更新、現在位置、再生状態、native backend とのやり取りは Rust backend 側で一元管理する。
- frontend は再生 command を起動し、Rust backend が持つ状態を表示する役割に留める。
- frontend 側で再生中判定や queue の正本を持たない。

### queue は順序つき配列として扱い、現在位置は index で識別する

- queue は song ID を含む entry の順序つき配列として扱う。
- 現在再生中の entry は song ID だけで識別せず、queue 上の current index で識別する。
- 同じ song ID が queue 内に複数回現れても、別 entry として区別する前提を採用した。

### OS 依存の再生処理は shim 境界に閉じ込める

- Rust backend には playback controller と backend interface を置き、その外側へ OS 依存の実装を漏らさない。
- v1 の再生実装は Windows backend のみを持つ。
- 他 OS の事情を frontend や command 層へ広げない。

### 音源取得は `stream` を基準にし、transcode は条件付きで扱う

- 再生用の音源取得は OpenSubsonic/Subsonic の `stream` endpoint を基準に扱う。
- raw で再生できる場合は raw を優先し、bitrate や format の指定が必要なときだけ transcode 条件を付与する。
- seek 用の offset は server capability を確認したうえで使う。

### server への再生報告は capability に応じて使い分ける

- local 再生そのものは server への再生報告がなくても成立する前提を採用した。
- `playbackReport` extension がある server では `reportPlayback` を使う。
- それ以外の server では `scrobble` を使う。
- どちらの報告方式も、local 再生制御とは分離して扱う。

### queue の server 同期は capability ベースで行う

- app 内の queue 状態を正本とし、server への queue 保存は同期機能として扱う。
- `indexBasedQueue` extension がある server では index-based queue API を使う。
- それ以外では通常の play queue API を使う。

## 判断理由

### Rust を正本にしたほうが frontend の責務と整合する

- このプロジェクトでは backend 中心で状態と制御を持ち、frontend は描画と入力受付を主に担当する構成を採用している。
- 再生状態まで frontend 側へ分散させると、queue、native backend、server 報告の整合が崩れやすい。

### current song を song ID だけで扱うと重複曲を区別できない

- queue に同じ曲を複数回入れた場合、song ID だけでは現在位置を一意に表せない。
- current index を正本にすると、next、prev、remove、restore の挙動を一貫して扱いやすい。

### Windows 先行でも shim 境界は最初から必要だった

- v1 を Windows 限定にしても、OS ごとの差分を上位層へ漏らすと後から分離し直す負担が大きい。
- 先に backend interface を置くことで、Windows 実装だけで始めても構造の崩れを防ぎやすい。

### 再生報告は UX や server 表示には関係するが、音を出す前提ではない

- `stream` endpoint は音源取得のための API であり、再生回数や now playing 更新は別の報告 API に分かれている。
- そのため、再生制御と server 報告を分離して扱う判断を採用した。

## 参照

- OpenSubsonic API overview: https://opensubsonic.netlify.app/docs/opensubsonic-api/
- stream: https://opensubsonic.netlify.app/docs/endpoints/stream/
- scrobble: https://opensubsonic.netlify.app/docs/endpoints/scrobble/
- reportPlayback: https://opensubsonic.netlify.app/docs/endpoints/reportplayback/
- getPlayQueueByIndex: https://opensubsonic.netlify.app/docs/endpoints/getplayqueuebyindex/
- savePlayQueueByIndex: https://opensubsonic.netlify.app/docs/endpoints/saveplayqueuebyindex/
