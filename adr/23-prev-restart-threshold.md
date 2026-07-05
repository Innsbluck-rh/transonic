<!--
author:   Claude
date:     2026-07-04 23:56
version:  0.0.1
-->

## prev の振る舞いを「閾値付きの頭出し」に変更

### 背景と決定

これまで `PlaybackController::prev`（`src-tauri/src/playback/controller.rs`）は再生位置を一切考慮せず、常に前のトラックへ移動していた。さらに `current_index == 0`（キューの先頭）では `Err("Already at the beginning...")` を返し、何も起きなかった。

一般的な音楽プレーヤー（Spotify、Apple Music、Media3 など）は「曲の頭付近なら前のトラックへ、数秒以上再生が進んでいたら現在の曲を頭に戻す」という二段階の挙動を採る。特に先頭トラックで prev を押しても頭に戻らないのは明確に直感に反する。そこで prev を次の挙動に変更した。

1. 再生中・一時停止中は先に backend から実位置を同期し、正確な再生位置で判定する。
2. `current_index == 0`（前のトラックが無い）または 再生位置 > 閾値 の場合は、現在の曲を先頭に戻す（`seek(context, 0)` を再利用）。
3. それ以外（頭付近かつ前トラックあり）は従来どおり前のトラックへ移動する。

閾値は `PREV_RESTART_THRESHOLD_MS = 3000`（3秒）の定数として定義した。これは Media3 の `maxSeekToPreviousPositionMs` のデフォルトや各社プレーヤーの事実上の標準に一致する。

### 3秒固定にした理由

閾値をユーザー設定にするか固定にするかを検討した。調査の結果、prev の判定はバックエンド非依存の純粋なコントローラ判定であり、全プラットフォームの prev が `controller.prev()` に集約されているため、閾値を設定値にすること自体は技術的に容易だと分かった。しかし設定 UI の形（自由入力か 3/5/10/15 のドロップダウンか等）を決めきれておらず、UI の認識コストと実装コストを避けるため、まずは 3 秒固定で入れることにした。定数化してあるので、後から設定値へ差し替えても全プラットフォームに一様に効く。

### プラットフォーム横断で1箇所に閉じられる理由

当初は「Android は Media3 が 3 秒閾値を組み込みで持つため、他プラットフォームだけ任意閾値にするのは面倒で、全体を 3 秒に揃えるしかない」と考えていた。しかし実際にコードを追うと、`PlaybackService.kt` は MediaSession に生の `ExoPlayer` ではなく `QueueCommandForwardingPlayer` ラッパーを渡しており、その `seekToPrevious()` / `seekToPreviousMediaItem()` は両方とも無条件に Rust の `"prev"` へ転送していた（`src-tauri/gen/android/app/src/main/java/com/innsb/transonic/playback/PlaybackService.kt`）。つまり Media3 組み込みの閾値ロジックは素通りしており、そのラッパーはキューを常に「現在の1曲だけ」として OS に見せることで、キューの権威を Rust 側に保っている（`adr/10` の方針どおり）。

この結果、`adr/20` #6 で指摘された「Android の3層スロット状態の権威二重化」問題は prev 閾値に関しては発生しない。閾値判定は Rust の `controller.prev()` ただ1箇所に置けばよく、デスクトップ（Symphonia）も Android（Media3→ラッパー→Rust転送）も同じ挙動になる。

Windows には現状 SMTC / メディアキーの OS 統合が Rust 側にもフロント側（`navigator.mediaSession`）にも存在せず、prev の入口はアプリ UI ボタン経由の Tauri コマンド一択なので、やはりコントローラに集約される。iOS / macOS / Linux は `backend_shims/unsupported.rs` の no-op で再生機能自体が未実装だが、将来 iOS を実装する場合も、ロック画面の「前へ」は `MPRemoteCommandCenter.previousTrackCommand` というアプリが自前で処理する素のインテントであり、OS が頭出しか前トラックかを押し付けることはない。したがってキューを OS プレーヤーへ丸投げしない限り（それは `adr/10` 違反）、閾値の自由は保たれる。

### テスト

`controller.rs` のテストを更新した。

- `next_prev_enforce_queue_boundaries` — 旧挙動（先頭で prev が `Err`）の前提を、新挙動（先頭では頭出しして `Ok`、index 据え置き）に合わせて更新した。
- `prev_restarts_current_track_past_threshold_and_steps_back_near_head`（新規）— 閾値超えで現在曲を頭出し（index 不変・`pending_seek_position_ms == Some(0)`）、頭付近で前トラックへ戻ることを固定した。
