# Cover Art Concerns

## 現状の懸念点

- フロント側の `coverArtCache` が成功 Promise を永続保持していて、Rust 側の 7 日 TTL を事実上バイパスしている。アプリを再起動しない限り再検証されないため、サーバ側のアート変更、ディスクキャッシュ削除、profile 削除後の壊れた URL などに弱い。

- `fetchCoverArtAssetUrl` の cache key には `profileId` が入っているが、Tauri command には `profileId` を渡しておらず、backend はその瞬間の active session を使っている。profile 切替と in-flight request が重なると、フロントの key と backend 実取得 profile がズレる余地がある。

- Rust 側に in-flight dedupe とディスクキャッシュがあるため、フロントの Promise cache は重複抑制目的としてはやや過剰になっている。むしろ成功結果の永久固定という副作用の方が目立つ。

- binary request 用の `ReqwestTransport` は request 構築に使われているが、実 fetch は `fetch_binary_response()` で毎回 `reqwest::blocking::Client::new()` している。機能上は動くが、接続再利用の面では少しもったいない。
