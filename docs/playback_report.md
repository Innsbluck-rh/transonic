# Playback Report

サーバー通知の送信タイミングは次で固定する。

## `reportPlayback` 対応サーバー

- `ignoreScrobble=false` で送信し、再生履歴 / play count の成立判定はサーバー側に任せる。
- `starting`: 曲のロード開始時。対象は初回 `play`、`next`、`prev`、`playQueueIndex`、自動次曲遷移、reload 必須 seek。
- `playing`: 実際に再生状態へ入った瞬間。対象はロード完了後の再生開始、`pause` からの `resume`、gapless 遷移。
- `paused`: `pause()` 成功直後。
- seek 後: seek 完了後の状態を `playing` または `paused` で 1 回送る。
- `stopped`: `stop()` 成功直後、手動で別曲へ移る直前、曲終端で次曲へ移る直前、曲終端で次曲がないとき。送る位置は `0` に戻す前の位置。
- heartbeat: `playing` 中だけ 15 秒ごとに `playing` を再送する。
- 送らない: `Interrupted`、buffering 中、`set_queue` だけ、状態復元だけ、`playback_get_state()` のポーリングだけ。

## `reportPlayback` 非対応サーバー

- `scrobble(submission=false)` は now playing 通知として使う。
- 送るのは曲の再生セッション開始時だけ。
- 対象は初回 `play`、`next`、`prev`、`playQueueIndex`、自動次曲遷移、gapless 遷移、`stop` 後に同じ曲を再度 `play` したとき。
- now playing 通知としては送らない: `pause`、`stop`、seek、heartbeat、buffering。
- `scrobble(submission=true)` は再生済み登録として使う。
- 送るのは曲の一時停止時、停止時、または別曲へ移る直前に、実再生時間が曲長の 50% 以上または 4 分以上に達しているとき。
- 曲長が不明な場合は、実再生時間が 30 秒以上に達しているとき。
