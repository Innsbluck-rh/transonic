author: ins

## 概要

WASAPIのノイズ原因となっていたマジでいらないリサンプリング処理の削除から学び、
オーディオ関連は「いらん事すな」を原則としていこうかと思った。

ついででなんかいらん事してる部分がないかざっくり聞いたなかでも特に気になったところを列挙。

## 自動 output stream recovery
[controller.rs (line 40)](C:/Users/innsb/Documents/transonic/src-tauri/src/playback/controller.rs:40) から始まる3回retry、[controller.rs (line 2253)](C:/Users/innsb/Documents/transonic/src-tauri/src/playback/controller.rs:2253) の文字列判定、[controller.rs (line 2461)](C:/Users/innsb/Documents/transonic/src-tauri/src/playback/controller.rs:2461) のschedule、[windows_runtime.rs (line 82)](C:/Users/innsb/Documents/transonic/src-tauri/src/playback/windows_runtime.rs:82) の遅延threadです。driver/device errorを裏で stop/reload/retry するので、状態遷移と再ロードがかなり増えています。音声まわりはfail-fastの方針なら削る価値が高いです。

- 3回retry、マジで謎。あまりにもマジックなナンバーではありませんか。
- その系列の話として他の項目も検討していらんなら"すな"、ということで。

## Raw stream失敗時のstandard fallback
[controller.rs (line 1650)](C:/Users/innsb/Documents/transonic/src-tauri/src/playback/controller.rs:1650) でraw開始し、失敗すると [controller.rs (line 1703)](C:/Users/innsb/Documents/transonic/src-tauri/src/playback/controller.rs:1703) からstandard streamを再要求します。ユーザーがRawを選んでいるのに、失敗時に別経路へ黙って落ちるので、品質や挙動の切り分けを濁します。RawはRawで失敗させる方がシンプルです。

- これ、standard streamが何なのかがよくわからｎ（Transcodingかそれに準ずる未指定ｃｏｄｅｃリクエストのこと言ってる？）
- ”RawはRawで失敗させる方がシンプルです。”　＞　普通に考えてそれはそうだと思う。同意。
