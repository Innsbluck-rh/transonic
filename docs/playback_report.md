# Playback Report

サーバー通知の送信タイミングは次で固定する。

## `reportPlayback` 対応サーバー

- `starting`: 曲のロード開始時。対象は初回 `play`、`next`、`prev`、`playQueueIndex`、自動次曲遷移、reload 必須 seek。
- `playing`: 実際に再生状態へ入った瞬間。対象はロード完了後の再生開始、`pause` からの `resume`、gapless 遷移。
- `paused`: `pause()` 成功直後。
- seek 後: seek 完了後の状態を `playing` または `paused` で 1 回送る。
- `stopped`: `stop()` 成功直後と、曲終端で次曲がないとき。送る位置は `0` に戻す前の位置。
- heartbeat: `playing` 中だけ 15 秒ごとに `playing` を再送する。
- 送らない: `Interrupted`、buffering 中、`set_queue` だけ、状態復元だけ、`playback_get_state()` のポーリングだけ。

## `reportPlayback` 非対応サーバー

- `scrobble(submission=false)` は now playing 通知として使う。
- 送るのは曲の再生セッション開始時だけ。
- 対象は初回 `play`、`next`、`prev`、`playQueueIndex`、自動次曲遷移、gapless 遷移、`stop` 後に同じ曲を再度 `play` したとき。
- 送らない: `pause`、`stop`、seek、heartbeat、buffering。
