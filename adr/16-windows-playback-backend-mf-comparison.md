<!--
author:   innsbluck / Claude
date:     2026-04-05 16:08
version:  0.0.1
-->

## 概要

- Windows 再生バックエンドの現行実装 (rodio/symphonia) で発見された制約と、代替となる Media Foundation (MF) の API レイヤー比較を記録します。
- 併せて、他の主要音楽プレイヤーにおける再生アーキテクチャの調査結果も記載します。

## 現行実装 (rodio) で判明した問題

### backward seek が機能しない

- 現行の Windows バックエンドは rodio 0.22.2 を使用しており、内部デコーダーは symphonia です。
- symphonia による backward seek（現在位置より前方へのシーク）が内部バグにより正常に動作しないことが判明しました。
- この問題のため、`WindowsPlaybackBackend::seek()` は常に `PlaybackSeekAction::ReloadRequired` を返す実装になっています。シーク操作のたびに HTTP からの全量ダウンロードとデコードのやり直しが発生します。

### 全量ダウンロード方式の制約

- 現行実装は `reqwest::blocking` で HTTP レスポンスの全バイトをメモリに読み込んでから rodio の Decoder に渡す download-then-play 方式です。
- 大きなファイルでは再生開始までの待ち時間が長く、メモリ消費も大きくなります。
- rodio に HTTP チャンクをストリーミングで流す改善案（`Read` トレイトを実装して逐次渡す方式）も検討しましたが、上記の backward seek バグが解消されない限り、シーク時の全量再取得は避けられません。rodio ベースでのストリーミング改善は費用対効果が低いと判断しました。

## Media Foundation の API レイヤー比較

Windows の Media Foundation には、再生パイプラインの抽象度が異なる複数の API レイヤーがあります。transonic の要件（HTTP ストリーミング再生、シーク、将来的な出力先の柔軟性）に照らして比較しました。

### IMFMediaEngine

HTML5 の `<audio>` 要素に相当する高レベル API です。Windows 8 以降で利用できます。

- URL を渡すだけで HTTP ストリーミング再生、バッファリング、シークを処理します。
- `SetSource(url)`, `Play()`, `Pause()`, `SetCurrentTime(seconds)` など、PlaybackBackend トレイトとほぼ 1:1 で対応する操作体系を持っています。
- 一方で、音声出力パスは内部の Streaming Audio Renderer (SAR) に固定されており、WASAPI 共有モードでの出力になります。出力デバイスや出力モードの変更はできません。
- ASIO 出力への切り替え、WASAPI 排他モードの使用、DSP チェーンの挿入（イコライザー、リプレイゲイン等）はいずれも不可能です。
- パイプラインの中間段階（デコード済み PCM）にアクセスする手段がありません。

### IMFMediaSession + Topology

MF のフルパイプライン制御 API です。Source Node → Transform Node → Output Node のトポロジーを自分で構築します。

- HTTP ストリーミングとシークは Source Resolver 経由で処理されます。
- カスタム Media Sink を実装すれば出力先を差し替えられますが、IMFMediaSink / IMFStreamSink の COM 実装は非常に複雑です。
- 非同期イベントループ (IMFAsyncCallback) の設計と COM スレッドモデルの管理が必要で、Rust からの実装量は最も大きくなります。
- 出力先の柔軟性を得るためにこの API を選ぶのであれば、IMFSourceReader のほうが同等の柔軟性をより少ない実装量で実現できます。

### IMFSourceReader

MF のデコード専用 API です。デマルチプレクスとデコードを MF に任せ、PCM サンプルを自分で受け取ります。

- `MFCreateSourceReaderFromURL(url)` で HTTP URL を直接開けます。MF 内部の Network Source が HTTP ストリーミングとバッファリングを処理します。
- `ReadSample()` を呼ぶと、デコード済みの PCM サンプルが `IMFSample` として返されます。
- `SetCurrentPosition()` でシーク可能です。MF が内部でストリーム位置の調整を行います。
- PCM サンプルを自分で受け取るため、出力先は完全に自由です。WASAPI（共有/排他）、ASIO、その他任意の出力 API にルーティングできます。
- PCM を受け取った後にデジタル信号処理を挿入することも自然にできます。
- 出力レイヤーは自前で実装する必要があります。cpal クレートを使えば WASAPI 出力を比較的少ない実装量で実現でき、cpal の `asio` feature flag で ASIO 出力にも対応できます。

### 比較表

| 評価項目 | IMFMediaEngine | IMFMediaSession | IMFSourceReader |
|---|---|---|---|
| HTTP ストリーミング | 内蔵 | 内蔵 | 内蔵 |
| シーク (backward 含む) | 可能 | 可能 | 可能 |
| 実装の複雑さ | 低〜中 | 高 | 中 |
| 音声出力先の変更 | 不可 | カスタム Sink で可能（高コスト） | 自由（PCM を自分で扱うため） |
| ASIO 出力 | 不可 | 高コストで可能 | 可能 |
| WASAPI 排他モード | 不可 | 高コストで可能 | 可能 |
| DSP チェーン挿入 | 不可 | カスタム MFT で可能 | 可能（PCM に直接処理） |
| ギャップレス再生 | 困難 | トポロジー切替が必要 | サンプル単位で制御可能 |
| reqwest の要否 | 不要 | 不要 | 不要 |
| COM 実装量 | IMFMediaEngineNotify 1 つ | IMFAsyncCallback + トポロジー構築 | 少ない（関数呼び出し中心） |

### 認証ヘッダーに関する注意点

Subsonic API の認証パラメータ (`u`, `p`, `t`, `s`) は URL クエリパラメータとして付与されるため、`MFCreateSourceReaderFromURL` にそのまま渡せる見込みです。ただし、カスタム HTTP ヘッダーが必要になる場合は、`IMFSourceResolver` とカスタム `IMFByteStream` の組み合わせで HTTP 取得を自前で行う必要が出てきます。この点は実装時に確認が必要です。

### Ogg Vorbis / Opus のコーデックカバレッジ

Media Foundation は Windows に同梱されたコーデックを使用します。MP3, AAC, FLAC, WMA, ALAC は標準で対応していますが、Ogg Vorbis と Opus (Ogg コンテナ) はデフォルトでは非対応です。Subsonic サーバーが Ogg Vorbis / Opus でトランスコードする構成の場合、MF 単体では再生できない可能性があります。現行の symphonia は Ogg Vorbis に対応しているため、これはカバレッジの後退になります。`stream` API で `format=raw` を指定して元フォーマットを取得するか、サーバー側のトランスコード設定に依存する形になります。

## 他の音楽プレイヤーにおける再生アーキテクチャの調査

主要なデスクトップ音楽プレイヤーがどのような再生パイプライン構造を採用しているかを調べました。

### foobar2000

- Input Component（デコーダー）→ DSP Chain → Output Component（レンダラー）の 3 段パイプラインです。各ステージはプラグインとして独立しています。
- デコーダーは PCM サンプルを出力するだけで、出力先を知りません。出力コンポーネントは PCM を受け取るだけで、入力ソースを知りません。
- 出力コンポーネントとして WASAPI (共有)、WASAPI (排他)、ASIO、DirectSound が選択可能です。すべてプラグインとして差し替えられます。
- この「デコードと出力の完全分離」が、foobar2000 がオーディオ愛好者に評価される設計上の基盤になっています。

### MusicBee / AIMP

- どちらも BASS audio library (un4seen.com) をコアに使用しています。
- BASS 自体が「デコード API」と「出力 API」を分離した設計になっており、デコード結果の PCM を WASAPI / ASIO / DirectSound のいずれにもルーティングできます。
- 構造としては foobar2000 と同様の「デコーダー → PCM → 出力」の分離パターンです。

### Winamp

- Input Plugin (in_*.dll) → DSP Plugin (dsp_*.dll) → Output Plugin (out_*.dll) のプラグインチェーン構造です。
- 1990 年代から「デコードと出力の分離」を実現していた設計です。

### VLC

- libvlc によるモジュール式パイプライン (access → demux → decode → aout) を採用しています。
- 各ステージがモジュールとして差し替え可能です。

### 共通して観察されるパターン

調査した全プレイヤーに共通して、デコーダーと出力レンダラーが PCM サンプルを境界として分離されている構造が見られました。デコーダーはフォーマット依存で PCM を生成する責務だけを持ち、出力レンダラーはデバイス依存で PCM を受け取る責務だけを持ちます。この境界があることで、ASIO / WASAPI / DirectSound 等の出力先を差し替えられる柔軟性が確保されています。

## 既存アーキテクチャとの適合性

- 現行の `PlaybackBackend` トレイト (`backend_shims/backend.rs`) は 6 メソッドの薄い境界であり、バックエンド実装の差し替えを想定した設計になっています。MF ベースの実装はこのトレイトを満たす新しいモジュールとして追加でき、コントローラーやフロントエンドへの影響はありません。
- `PlaybackSeekAction` enum に `Applied` バリアントが既に定義されているため、MF バックエンドがネイティブシークに成功した場合はこれを返すだけでコントローラー側の処理が自然に切り替わります。
- `backend_shims/mod.rs` の `#[cfg]` ベースのディスパッチ構造により、現行の rodio 実装を残したまま MF 実装を並行して開発し、切り替えることが可能です。
- 現行のワーカースレッドパターン（`std::sync::mpsc` による RPC）は、MF の COM オブジェクトが要求する STA (Single-Threaded Apartment) モデルとも整合します。
