<!--
author:   innsbluck / Claude
date:     2026-04-05 20:30
version:  0.0.1
-->

## 概要

- Windows 再生バックエンドの symphonia 実装において、全量ダウンロード方式（download-then-play）をプログレッシブ・ストリーミング方式に置き換えた経緯と実装内容を記録します。
- 併せて、MF（Media Foundation）/ symphonia / HTML Audio の各バックエンドについて調査・比較した結果を記載します。

## 背景

ADR-16 で記録した通り、rodio/symphonia バックエンドは HTTP レスポンスの全バイトをメモリにダウンロードしてからデコードを開始する download-then-play 方式でした。ADR-16 の時点では MF（IMFSourceReader）への移行が検討されていましたが、その後 MF の FLAC シーク（`SetCurrentPosition`）が正常に動作しないバグが判明し、MF バックエンドの採用を一旦保留しました。

暫定措置として rodio を介さず symphonia を直接使用する `windows_symphonia` バックエンドを作成し、メモリ内シーク（`PlaybackSeekAction::Applied`）を実現しましたが、download-then-play 方式は変わっていませんでした。

この方式の問題は大きなファイルで顕著でした。184MB（約2時間）の MP3 ライブミックスの再生開始に約10秒を要しており、実用上の許容範囲を超えていました。

## MF の FLAC シークバグについて

Media Foundation の FLAC Media Source における `SetCurrentPosition` は、指定位置に関わらず常にストリーム先頭（position 0）から再生を開始する不具合があります。

- NAudio Issue #628 や .NET MAUI のレポートで同様の現象が報告されています。
- Microsoft の FLAC 対応に関する公式ドキュメントページは 404 を返す状態であり、FLAC サポート自体が非公式または不完全な位置づけであると推測されます。
- この不具合は MF の内部実装に起因するものであり、API 利用側から回避する手段はありません。

MF は FLAC 以外のフォーマット（MP3, AAC 等）では HTTP ストリーミング・シーク・PCM 出力の全てが正常に動作するため、FLAC だけのために MF を全面的に放棄するかどうかは判断が難しい状況です。

## 各バックエンド方式の比較

### MF（IMFSourceReader）

- HTTP ストリーミングは MF 内部の Network Source が処理するため、再生開始は高速です。
- シークは `SetCurrentPosition` で処理され、FLAC 以外では正常に動作します。
- PCM 出力を自分で受け取るため、WASAPI 排他モード・ASIO・DSP チェーン挿入が可能です。
- FLAC シークバグのため、FLAC ファイルのシークが機能しません。

### symphonia（download-then-play、旧方式）

- `reqwest::blocking` で HTTP レスポンスの全バイトを `Vec<u8>` にダウンロードし、`Cursor<Vec<u8>>` を symphonia の `MediaSourceStream` に渡す方式です。
- ダウンロード完了後は symphonia の SeekTable を使ったシークが FLAC を含む全フォーマットで動作します。
- 再生開始がファイルサイズに比例して遅延します（184MB で約10秒）。
- rodio 経由の旧バックエンド（`windows_rodio`）と本質的に同一の方式です。rodio 版は `seek()` が常に `ReloadRequired` を返すためシーク毎に再ダウンロードが発生していましたが、symphonia 版はメモリ内シーク（`Applied`）を返す点が改善でした。

### HTML Audio（HTMLAudioElement）

- Tauri の WebView2（Chromium）が内蔵する `<audio>` 要素を使う方式です。Feishin の web プレイヤーがこの方式を採用しています。
- Chromium の ffmpeg デコーダが HTTP ストリーミング・バッファリング・シーク（HTTP Range request 経由）を自動で処理します。
- FLAC を含む主要フォーマット（MP3, AAC, FLAC, Ogg Vorbis, Opus, WAV）に対応しており、MF より広いコーデックカバレッジを持ちます。
- 音声出力パスは WebView2 のオーディオレンダラに固定されるため、WASAPI 排他モード・ASIO 出力は不可能です。
- ギャップレス再生は `HTMLAudioElement` の仕様上サンプル精度では実現できません。Feishin は 2 つの `HTMLAudioElement` インスタンスを交互に使用する方式で近似的なギャップレスを実装していますが、50〜120ms 程度のギャップが残ります。

### 比較表

| 評価項目 | MF (SourceReader) | symphonia (download) | HTML Audio | symphonia (streaming) |
|---|---|---|---|---|
| 再生開始速度 | 高速 | ファイルサイズに比例して遅い | 高速 | 高速 |
| FLAC シーク | 不可（MF バグ） | 可能 | 可能 | 可能 |
| WASAPI 排他 / ASIO | 可能 | 可能 | 不可能 | 可能 |
| DSP チェーン挿入 | 可能 | 可能 | Web Audio API の範囲内 | 可能 |
| ギャップレス再生 | サンプル単位で制御可能 | サンプル単位で制御可能 | 近似のみ（50〜120ms） | サンプル単位で制御可能 |
| 追加依存 | windows crate (既存) | なし | なし | なし |
| Ogg Vorbis / Opus | 非対応 | 対応 | 対応 | 対応 |

## symphonia のストリーミング対応に関する調査

### symphonia は download-then-play を強制しない

symphonia の `MediaSourceStream` は `Box<dyn MediaSource>` を受け取ります。`MediaSource` トレイトは `Read + Seek + Send + Sync` を要求し、`is_seekable()` と `byte_len()` の 2 メソッドを定義しています。

symphonia には `ReadOnlySource<R>` というアダプターが存在し、`Read + Send + Sync` のみを実装する型を非シーカブルな `MediaSource` としてラップできます。このアダプターは `is_seekable()` が `false` を返し、`Seek::seek()` はエラーを返すスタブ実装です。`MediaSourceStream` は非シーカブルソース用のバッファキャッシュも備えており、`ReadOnlySource` を使ったストリーミング再生自体は可能です。

ただし `ReadOnlySource` を使うとシークが不可能になります。symphonia の FLAC シークは SeekTable からバイトオフセットを算出し `Seek::seek()` で該当位置に移動する方式であるため、ソースがシーカブルでなければ機能しません。FLAC シークの実現が symphonia 移行の主目的であったため、`ReadOnlySource` の採用は目的と矛盾します。

### カスタム MediaSource による両立

`MediaSource` はトレイトであるため、独自の型で実装すればストリーミングとシークを両立できます。symphonia の各フォーマットリーダーがファイルに対して行う I/O パターンを分析した結果、以下のことが確認できました。

- **プロービング（フォーマット検出）**: FLAC ではファイル先頭の `fLaC` マーカーとメタデータブロック（StreamInfo, SeekTable 等）を順次読み出すだけであり、ファイル全体を必要としません。数 KB〜数十 KB の先頭データがあればプロービングは完了します。
- **`byte_len()` の要件**: HTTP レスポンスの `Content-Length` ヘッダから取得できます。
- **`SeekFrom::End(0)` の要件**: ファイル全体のバイト長を返すだけであり、I/O は不要です。`Content-Length` の値で応答できます。
- **FLAC シーク時の I/O パターン**: SeekTable（プローブ時にメモリに読み込み済み）でバイト範囲を絞り、バイナリサーチで中間地点に `seek()` → フレームヘッダを数百バイト読む → タイムスタンプを確認、を繰り返します。SeekTable のエントリ間隔が十分に細かい場合、バイナリサーチは 2〜3 回のイテレーションで収束します。

## 実装した内容

### StreamingMediaSource

`symphonia::core::io::MediaSource` を実装するカスタム型 `StreamingMediaSource` を `windows_symphonia.rs` 内に追加しました。

- バックグラウンドの HTTP ダウンロードスレッドが `reqwest::blocking::Response` のボディを 64 KiB チャンクで読み取り、`SharedDownloadBuffer` に逐次追記します。
- `StreamingMediaSource` はこのバッファから `Read` を実装し、要求位置のデータがまだダウンロードされていない場合は `Condvar` で待機します。
- `Seek` はバッファ内の任意の位置への移動を即座に行います。未ダウンロード位置への `seek()` も受け入れ、次の `read()` がデータ到着まで待機する形で動作します。
- `is_seekable()` は `true` を返し、`byte_len()` は `Content-Length` から取得した値を返します。
- `Content-Length` ヘッダが存在しない場合は、従来通りレスポンス全体をダウンロードして `Cursor<Vec<u8>>` を使うフォールバックパスを用意しています。

### do_load の変更

`do_load` 関数の処理フローを変更しました。

1. HTTP GET リクエストを送信し、レスポンスヘッダ（`Content-Length`）を確認します。
2. `Content-Length` が存在する場合、`SharedDownloadBuffer` を作成し、ダウンロードスレッドを起動して `StreamingMediaSource` を構築します。
3. `StreamingMediaSource` を `Box<dyn MediaSource>` としてポンプスレッドに渡します。
4. ポンプスレッドはヘッダの数 KB が到着した時点でプロービングを完了し、デコードを開始します。

### pump_entry / pump_init_and_run の変更

引数を `bytes: Vec<u8>` から `source: Box<dyn symphonia::core::io::MediaSource>` に変更し、内部で `Cursor::new(bytes)` を生成する処理を削除しました。ストリーミングモードでもフォールバックモードでも同じコードパスでデコードが動作します。

### ActiveSession / tear_down の変更

`ActiveSession` に `download_handle: Option<JoinHandle<()>>` と `download_buffer: Option<Arc<SharedDownloadBuffer>>` を追加しました。`tear_down` では `SharedDownloadBuffer::cancel()` を呼び出してダウンロードスレッドを停止し、`Condvar::notify_all()` で `Read` の待機を解除した上で、ダウンロードスレッドを join します。

## 動作確認結果

以下のファイルで再生開始とシークの動作を確認しました。

| ファイル | サイズ | フォーマット | 再生開始 | シーク |
|---|---|---|---|---|
| 平沢進 / Aria in a Circuit Board | 2.3 MB | MP3 | ストリーミングモードで即座に開始 | forward / backward ともに `Applied` |
| 核P-MODEL / OPUS | 38.6 MB | FLAC | ストリーミングモードで即座に開始 | ダウンロード完了前（1.4秒時点で138秒地点へ）のシークも成功 |
| 核P-MODEL / HUMAN-LE | 32.9 MB | FLAC | ストリーミングモードで即座に開始 | トラック切替時にダウンロードが 11.4 MB 地点でキャンセルされた（正常動作） |
| 核P-MODEL / ECHO-233 | 42.4 MB | FLAC | ストリーミングモードで即座に開始 | ダウンロード進行中に 222 秒地点へシーク → 成功、ダウンロード完了後に backward シーク → 成功 |
| 田中フミヤ / Live Mix (2時間) | 184 MB | MP3 | ストリーミングモードで即座に開始（従来は約10秒待ち） | 確認済み |
| Shpongle / Live In Eilat (2時間) | 184.9 MB | MP3 | ストリーミングモードで即座に開始 | 確認済み |

ログ上、全てのファイルで `streaming mode, content_length=...` が記録され、プロービング完了（`probed format ch=2 sr=44100`）がダウンロード完了前に発生していることを確認しました。

## 現在の実装にできていないこと

以下は実装されていない制約の記録です。

### 未ダウンロード位置へのシーク時のレイテンシ

現在の実装は、シーク先のバイト位置がまだダウンロードされていない場合、バックグラウンドの順次ダウンロードがその位置に到達するまで `Condvar` で待機します。HTTP Range リクエストによる部分取得は実装されていません。

これは以下の状況で影響します。

- 大きなファイルのダウンロード序盤に、ファイル後半へシークした場合。例えば 184 MB のファイルでダウンロードが 20 MB まで進んだ時点で 150 MB 地点（終盤）にシークすると、残り 130 MB のダウンロードが完了するまで再生が再開されません。
- 実測では、ローカルネットワーク上の Navidrome に対して 184 MB のファイル全体のダウンロードは約10秒であったため、最悪ケースでもその範囲内に収まります。回線が遅い環境ではこの影響が大きくなります。

Range リクエストによる解決は技術的に可能ですが、Subsonic / OpenSubsonic API の仕様には `stream` エンドポイントにおける HTTP Range ヘッダの挙動について規定がありません。`format=raw` でファイルをそのまま配信する場合はサーバー側の HTTP フレームワーク（Navidrome は Go 標準ライブラリ、Airsonic-advanced は Spring Boot）が Range を自動処理する可能性が高いですが、トランスコード時は Range が適用できません。この前提の不確実性から、現時点では Range リクエストを使用していません。

### M4A (ISO MP4) の moov アトム配置問題

ISO MP4 コンテナの `moov` アトム（メタデータ・サンプルテーブル）はファイル末尾に配置されている場合があります。symphonia の ISO MP4 デマルチプレクサは `moov` を読むためにファイル末尾へシークする可能性があり、ストリーミングモードではその位置がまだダウンロードされていないため、データ到着まで待機が発生します。ストリーミング最適化された MP4（`moov` が先頭にある）ではこの問題は発生しません。Subsonic サーバーが配信する M4A ファイルの `moov` 配置はサーバーやエンコーダーの設定に依存します。

### Content-Length なしのレスポンス

HTTP レスポンスに `Content-Length` ヘッダが含まれない場合（`Transfer-Encoding: chunked` 等）、`byte_len()` に返す値が得られないため、ストリーミングモードを使用せず従来通りの全量ダウンロード方式にフォールバックします。Subsonic サーバーの `stream` エンドポイントが `Content-Length` を返すかどうかはサーバー実装に依存しますが、`estimateContentLength=true` パラメータを付与することでヘッダの付与を促すことが可能です（トランスコード時は推定値）。現在のリクエストではこのパラメータを付与していません。

### バッファのメモリ消費

`SharedDownloadBuffer` はダウンロードされたデータを `Vec<u8>` に全て保持します。ダウンロードが完了するとファイル全体がメモリ上に存在する状態になり、これは旧方式と同じメモリ消費量です。ストリーミング方式ではあるものの、既に再生済みの領域を解放する仕組みは実装されていません（backward シークのためにバッファ全体を保持する必要があるため）。

### download-then-play 方式との振る舞いの差異

ストリーミング方式では、ダウンロード中のエラー（ネットワーク断等）が再生中に発生する可能性があります。旧方式ではダウンロード完了を待ってから再生を開始するため、ダウンロードエラーは再生開始前に検出されていました。ストリーミング方式では、再生中にダウンロードスレッドがエラーを報告すると、ポンプスレッドの次回 `Read` 呼び出しでエラーが返され、デコードが中断します。
