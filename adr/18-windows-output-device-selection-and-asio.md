<!--
author:   Codex
date:     2026-06-19 18:25
version:  0.0.1
-->

## 2026-06-20 修正

- ~~この ADR に記録した ASIO runtime integration を、Windows 出力デバイス切り替えの現行実装として扱います。~~\
  (2026-06-20 17:40) この方針は ADR-19 で取り下げました。Windows の出力デバイス切り替えは WASAPI 限定に戻し、ASIO runtime integration はスクラップしました。
- ~~`PlaybackOutputDevice.host`、`PlaybackCapabilities.asioOutput`、ASIO host 列挙、ASIO device 解決、ASIO pause/stop/drop 特別処理、ASIO UI group、ASIO runtime test を出力デバイス切り替えの実装として維持します。~~\
  (2026-06-20 17:40) これらは削除対象です。現行実装や再実装の根拠として参照しないでください。
- `windows-asio` Cargo feature、`asio-sys` patch crate、ASIO SDK / binary bundle / build support に関する記述は runtime 実装ではありません。これらが残っていても、ASIO 出力機能が残っていることを意味しません。
- ASIO を再度扱う場合、この ADR の実装を継ぎ足すのではなく、ADR-19 に記録した通り別機能としてスクラップアンドビルドします。

## 概要

- Windows の再生出力先を設定値として保持し、CPAL の出力デバイス解決に渡すための実装を追加しました。
- Windows の通常出力は WASAPI を既定経路として扱い、ASIO は Cargo feature `windows-asio` による opt-in 実装として追加しました。
- Windows の `windows_symphonia` バックエンドで、選択された CPAL output device を使って出力ストリームを作成する実装を追加しました。
- ASIO 選択時は、ASIO ドライバが報告する出力構成に合わせて CPAL stream を構築する実装を追加しました。

## 目的

- Windows でユーザーが出力デバイスを指定するための設定値、API、UI、再生バックエンド処理を接続することを目的としました。
- システム既定デバイスを `null` として扱い、明示的なデバイス選択では CPAL の `DeviceId` 文字列表現を保存することを目的としました。
- ASIO 出力を通常ビルドから分離し、`windows-asio` feature を指定したビルドだけで ASIO host を列挙・選択することを目的としました。
- ASIO ドライバの出力 sample format、sample rate、channel count に合わせて出力ストリームを作成することを目的としました。

## 設定モデル

- `PlaybackSettings` に `outputDeviceId: string | null` を追加しました。
- `null` はシステム既定出力デバイスを表します。
- 空文字列または空白のみの `outputDeviceId` は、設定ロード時および設定置換時に `None` / `null` へ正規化します。
- 既存設定ファイルに `outputDeviceId` が存在しない場合は、既定値として `null` を使用します。
- 明示的な出力デバイス指定では、CPAL の `DeviceId` を `Display` で文字列化した値を保存します。
- 設定更新時、非 `null` の `outputDeviceId` は保存前に playback layer の `validate_output_device_id` で検証します。

## 型定義とコマンド

- `PlaybackOutputDevice` を追加しました。

```ts
type PlaybackOutputDevice = {
  id: string
  name: string
  host: "wasapi" | "asio"
  isSystemDefault: boolean
}
```

- `playback_get_output_devices() -> Vec<PlaybackOutputDevice>` を追加しました。
- `PlaybackCapabilities` に `outputDeviceSelection: boolean` と `asioOutput: boolean` を追加しました。
- Windows の `windows_symphonia` backend では `outputDeviceSelection` を `true` とし、`asioOutput` は `cfg!(feature = "windows-asio")` の値を返します。
- 非対応 backend では出力デバイス一覧を空配列として返し、出力デバイス選択 capability を無効値として扱います。

## Cargo feature と ASIO build

- `src-tauri/Cargo.toml` に `windows-asio = ["cpal/asio"]` を追加しました。
- `windows-asio` は default feature に含めていません。
- ASIO device の列挙は `#[cfg(feature = "windows-asio")]` の内側に置きました。
- `src-tauri/crates/asio-sys` に `asio-sys` のローカル patch crate を追加しました。
- `src-tauri/Cargo.toml` の `[patch.crates-io]` で `asio-sys` をローカル patch crate に向けました。
- ローカル `asio-sys` の `build.rs` では、bindgen に対して `layout_tests(false)` を指定しました。
- ローカル `asio-sys` の `build.rs` では、`LIBCLANG_PATH` が未設定の場合に `NDK_HOME`、`ANDROID_NDK_HOME`、`ANDROID_HOME` から libclang の候補パスを設定する処理を追加しました。
- ローカル `asio-sys` の `build.rs` では、bindgen の clang args に `--target={TARGET}` を追加しました。
- `src-tauri/Cargo.toml` の workspace 設定では `crates/asio-sys` を workspace 対象外にしました。

## 出力デバイス列挙

- Windows では CPAL の WASAPI host を常に列挙対象にしました。
- `windows-asio` feature が有効なビルドでは、CPAL の ASIO host も列挙対象にしました。
- 列挙結果では CPAL device の `id()` を `PlaybackOutputDevice.id` として保存します。
- 列挙結果では CPAL device の `description().name()` を表示名として使用し、取得できない場合は `DeviceId` 文字列を表示名として使用します。
- `isSystemDefault` は WASAPI host の default output device と一致する場合だけ `true` にします。
- ASIO host には Windows system default の概念を割り当てず、ASIO device は default 表示対象にしません。
- 出力デバイス一覧は WASAPI、ASIO の順に並べ、同一 host 内では表示名と id でソートします。

## 出力デバイス解決

- `resolve_cpal_output_device(outputDeviceId)` を追加しました。
- `outputDeviceId` が `None` / `null` の場合は WASAPI host の `default_output_device()` を使用します。
- `null` は Windows system default WASAPI を表し、CPAL default host や ASIO default-like device には解決しません。
- `outputDeviceId` が指定されている場合は、`cpal::DeviceId::from_str` で parse し、`DeviceId` に含まれる `HostId` から `cpal::host_from_id` を呼びます。
- host 解決後、`host.device_by_id(&device_id)` で CPAL device を取得します。
- `windows-asio` feature が無効なビルドで `asio:` の device id が指定された場合は、ASIO feature なしのビルドであることを示す error を返します。

## 再生コントローラー

- `PlaybackBackend` trait に `set_output_device(output_device_id)` を追加しました。
- Windows symphonia backend の active worker に `SetOutputDevice` job を追加しました。
- active worker は選択中の `output_device_id` を保持し、次回 `Load` または `ActivatePrepared` の `activate_session` に渡します。
- `PlaybackController::set_output_device` を追加しました。
- stopped 状態では、backend に `output_device_id` を渡し、再生ストリームの reload は行いません。
- playing または paused 状態では、現在の track index と再生位置を取得し、prepared gapless state を消去し、選択デバイスを backend に渡した上で同一 track を取得済み位置から再ロードします。
- 切り替え前が playing の場合は autoplay 付きで再ロードし、paused の場合は paused 状態で再ロードします。
- device 設定を backend に渡す段階で error が発生した場合、controller は既存 stream を停止せず、既存の再生状態を維持します。
- active stream の reload 後に error が発生した場合、controller は `Playing` / `Paused` に戻さず `Error` へ遷移し、出力デバイス設定を前値へ戻します。
- app exit 時は playback state を保存した後、backend を明示的に stop して ASIO/WASAPI stream を通常の teardown 経路で解放します。
- `settings_update` は出力デバイス反映に失敗した設定を成功扱いせず、保存済み設定を前値へ rollback して error を返します。

## Windows symphonia backend の CPAL stream

- `build_cpal_stream` に `output_device_id` を渡すようにしました。
- WASAPI、システム既定出力、ASIO のいずれでも、選択された CPAL device の `default_output_config()` と `supported_output_configs()` を参照して CPAL output config を選択します。
- source channel count 以下かつ device default output channel count 以下の channel count を target channel count として扱います。
- source sample rate と target channel count が supported output config に含まれる場合、その config を使用します。
- source sample rate の config が取得されない場合は、device default sample rate と target channel count の config を使用します。
- default sample rate の config が取得されない場合は、target channel count の supported config から default sample rate に近い sample rate の config を使用します。
- supported output config が取得されない場合は、`default_output_config()` を使用します。
- 選択した CPAL `SampleFormat` に応じて `I8`、`I16`、`I24`、`I32`、`I64`、`U8`、`U16`、`U24`、`U32`、`U64`、`F32`、`F64` の typed output stream を構築します。
- DSD 系およびその他の sample format は、この backend の output sample format として扱わず error を返します。
- CPAL callback では内部 PCM を f32 buffer として生成し、typed output stream に渡す直前に `cpal::FromSample<f32>` で出力 sample 型へ変換します。
- 変換前の f32 sample は `[-1.0, 1.0]` に clamp します。
- WASAPI/ASIO 共通 callback では source sample rate と output sample rate が異なる場合に線形補間で resampling します。
- WASAPI/ASIO 共通 callback では source channel count と output channel count が異なる場合に channel mapping を行います。
- source が stereo 以上で output が mono の場合は、source frame の全 channel を平均して mono にします。
- output channel count が source channel count より大きい場合は、source channel が存在する channel だけを書き込み、残りの output channel は無音にします。
- source が mono の場合は、output channel に mono sample を配置します。
- gapless handoff 時は output render cursor を reset します。
- ASIO の pause では CPAL stream に `pause()` を呼ばず、stream を running のまま pause-silence mode に切り替えて出力 buffer を無音で埋めます。
- pause-silence mode 中の callback は PCM ring と pending gapless state を消費・破棄せず、resume 時に同じ位置から通常 render へ戻します。
- stop / clear / active session 差し替え時は CPAL stream を pause する前に stop-silence mode に入り、残留 PCM を破棄します。
- ASIO session の teardown では、CPAL ASIO callback が実際に ASIO buffer を無音化できるよう、短い timeout 内で複数 callback を待ちます。
- ASIO session の active stream drop 後は、同じ ASIO device に短い silence flush stream を作成し、driver 解放前の最終出力 buffer を無音で上書きします。
- ASIO stream drop 後は、driver/buffer 解放が WASAPI 再初期化や他アプリの出力へ衝突しないよう、短い release settle を置きます。
- stop-silence mode 中の callback は gapless handoff と EOS 通知を行わず、出力 buffer 全体をゼロで埋めます。
- WASAPI/ASIO stream 作成時、source channels、source sample rate、output channels、output sample rate、sample format を log に記録します。
- 出力デバイス解決、config 取得、stream build、非同期 stream error は `Audio output device error:` prefix の message に揃え、controller が raw/fallback の音源取得失敗と区別できるようにします。
- CPAL stream error callback では `BufferUnderrun` を recoverable として log に留め、`DeviceNotAvailable`、`StreamInvalidated`、backend specific error は `PlaybackNativeEvent::Error` として controller へ一度だけ通知します。
- controller は `Audio output device error:` かつ output stream lost/failed の native error を受けた場合、現在の queue index と再生位置を保持し、既存 backend stream を停止してから復旧予定を `Interrupted` 状態で保持します。
- Windows runtime は復旧予定の backoff 経過後に native event processing を再 dispatch し、controller は同一 track/position を最大 3 回まで再ロードします。
- 自動再ロードが成功した場合は元の `Playing` / `Paused` 状態へ戻し、retry 上限到達または active session 欠落時は `Playing` に残さず `Error` 状態へ遷移します。
- ユーザー操作による通常の load/stop/queue 更新後は retry count と pending recovery をリセットします。
- 保存済み device から system default WASAPI への一時 fallback は行いません。保存済み WASAPI / ASIO device が unavailable の場合も、保存値は維持し、retry 上限後に明示的な `Audio output device error:` として扱います。
- 起動直後 ASIO device の一時 unavailable と恒久的な device missing は v1 では区別しません。保存済み出力先を勝手に破棄したり、ユーザーが選んでいない system default WASAPI へ無音で切り替えたりしないことを優先します。

## UI

- Windows の playback settings に output device selector を追加しました。
- selector には `System Default (WASAPI)` option を追加し、値は `null` に対応させました。
- device option は `WASAPI` と `ASIO` の host label で分類します。
- WASAPI の system default device だけに `(System Default)` を付け、ASIO device には default 表示を付けません。
- 保存済み device id が列挙結果に存在しない場合、`Unavailable device` option を現在値として表示します。
- selector の変更時、空文字列は `null` に変換し、非空文字列はそのまま `setPlaybackSetting("outputDeviceId", id)` に渡します。
- playback capability の `outputDeviceSelection` が無効な場合は selector を表示しません。
- dropdown header の背景色指定を固定色から変更し、container 側の背景と text color を theme に従わせる style にしました。

## テスト

- settings の Rust test に `outputDeviceId` の missing、empty string、persist、normalization の検証を追加しました。
- playback controller の Rust test に stopped、playing、paused、prepared gapless clear、backend error、reload error rollback の device change 検証を追加しました。
- playback controller の Rust test に、出力デバイスエラーでは raw stream 失敗後に standard stream fallback を行わない検証を追加しました。
- playback controller の Rust test に、native output stream error 後の backoff 付き再初期化、複数 retry、retry 上限、reload 一時失敗後の成功、reload 継続失敗時に stream なし `Playing` へ残らない検証を追加しました。
- Windows symphonia backend の Rust test に ASIO output mapping、linear resampling、stop-silence mode、WASAPI 共通 output config 選択、CPAL stream error event 化の検証を追加しました。
- frontend test に default settings、output selector の表示、device selection、system default selection、unavailable saved device 表示の検証を追加しました。
- `src/features/settings/service.test.ts` に `outputDeviceId` の normalization と persistence の検証を追加しました。

## 実行した検証

- `cargo check --manifest-path src-tauri\Cargo.toml --no-default-features --features windows-asio`
- `$env:CARGO_TARGET_DIR='src-tauri\target\codex-asio-build'; cargo build --manifest-path src-tauri\Cargo.toml --no-default-features --features windows-asio`
- `pnpm cargo:check`
- `pnpm cargo:test`
- `pnpm bindings:check`
- `pnpm test`
- `pnpm format:check`
