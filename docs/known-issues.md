# 実装負債バックログ

## 位置づけ

- これは「生きた」優先順位付きバックログです。`adr/`とは役割が異なります。
  - `adr/20-implementation-garbage-audit.md` は 2026-06-20 時点の監査記録であり、**変更しません**（ADRは過去の意思決定・調査記録専用というプロジェクトルールのため）。
  - このファイルは、その監査結果を出発点に、優先度・状態・対応方針を随時書き換えていくための実務用ドキュメントです。今後の予定を書いてよいのはここだけです。
- 新しく見つけた負債もここに直接追記してよい。ADRを新設する必要はありません。
- 対応が終わった項目は削除せず、末尾の「解決済み」に日付付きで移動する。

- 追補(ins):これはClaude Codeによって既存の散らばったtodoをもとに生成されたファイルである。この追補のように(ins)とあるものはコントリビュータが追記した文である。

## 優先度Tierの考え方

TODOの「量」ではなく、**先送りするほど複利的にコストが増えるかどうか**（何層・何言語にまたがっているか）を主軸にしている。次にどの機能領域を伸ばすかがまだ決まっていない前提なので、特定機能への近さではなく構造的な広がりで並べた。

- **Tier 1**: 3つ以上の言語/リポジトリ境界にまたがって同じ概念を別々に持っている。関連領域を触るたびに複数箇所の同期が必要で、コストが複利的に増える。着手前に「決定」だけでもしておく価値が高い。
- **Tier 2**: フロントエンド⇄Rustバックエンドの2層にまたがる。言語境界は1〜2個だが、UIの見た目や挙動に直接影響する。
- **Tier 3**: 単一レイヤー・単一クレート内に閉じているが、行数や責務が肥大化している。閉じている分、後回しにしても他領域への波及は少ない。
- **Tier 4**: 影響範囲が狭い、または現状実害がない（例: まだ使われていない抽象）。緊急度は低い。

---

## Tier 1

### Android playbackのmedia slot状態が三重管理
状態: 未着手 / 元: adr/20 #6

Rust (`prepared_generation`・`media_instance_id`)、Kotlin `AndroidPlaybackPlugin.kt` (`ControllerMediaState`)、Kotlin `PlaybackService.kt` (`PlaybackSlotState`)がそれぞれcurrent/prepared/transitioningの状態を持つ。`basePositionMs`の所有も分裂している。gapless遷移・position計算・手動遷移・通知操作のバグ温床になりやすい。

対応方針メモ: Android実機での再生バグ調査が発生した際、まずどの層のどの状態が食い違っているかを毎回突き止めるコストが高い。次にAndroid再生周りのバグ修正が発生したタイミングで、正本を1層に決める設計整理とセットで行うのが効率的か。

---

## Tier 2

### playback commandのIPC契約が実態と合っていない
状態: 未着手 / 元: adr/20 #1

多くのplayback commandは`Result<(), String>`を返すが、実際は`spawn`/`spawn_blocking`で非同期に投げてすぐ`Ok(())`を返す。失敗はbackground taskのログに流れるだけ。フロントの`hasPlaybackCommandError()`はこの見かけ上の契約を前提にしている。加えて`SongList.tsx`・`QueueList.tsx`が`usePlayback()`facadeを通さず`commands.playbackSetQueue()`等を直呼びしている。

対応方針メモ: 「即完了系command」と「fire-and-forget系command」を型で区別する/component からの直呼びをfacade経由に統一する、の2つは分離して着手できる。

### remote Connect shared playbackがlocal PlaybackStatusに偽装されている
状態: 未着手 / 元: adr/20 #3

`mergePlaybackStatusWithSharedPlayback()`でremoteの共有再生状態をlocalの`PlaybackStatus`型に合成し、存在しないdiagnosticsを`null`等で埋めている。UI側はlocal/remoteを別関数で判定する必要があり、テストもこの偽装を固定している。

対応方針メモ: `LocalPlaybackStatus | RemoteSharedPlaybackStatus`のような判別可能な型に分ける設計変更。Tier 1のConnect queue意味論整理と合わせて着手すると手戻りが少ないかもしれない。

---

## Tier 3

### Windows Symphonia backendが単なるshimではなく巨大な再生エンジンになっている
状態: 未着手 / 元: adr/20 #5

`src-tauri/src/playback/backend_shims/windows_symphonia.rs`にHTTPダウンロードバッファ・Symphonia MediaSource・codec registry・decode pump・seek・ring buffer・resampling・gapless router・worker thread・出力デバイス選択・CPALコールバック・エラー分類が集中している。

対応方針メモ: 単一ファイル・単一層に閉じているため急ぎではないが、Windows再生関連の変更のたびに読む範囲が広がる。責務分割は独立して進められる。

### opensubsonic-client crateのresponse型境界が未完了
状態: 未着手 / 元: adr/20 #7

設計（`docs/opensubsonic-client-design.md`）ではlow-level clientがOpenSubsonic/Subsonicのrequest/responseを扱う想定だが、browsing/lists/searchでは`serde_json::Value`のまま返しており、`RawSong`等の互換パースがcommand層(`src-tauri/src/commands/browse/*`)に漏れている。

対応方針メモ: API追加のたびにcommand側のRaw型とparserが増え続ける構造。新しいbrowsing系endpointを追加する予定が出たタイミングで着手するのが自然。

### Windows出力ストリームの自動リカバリが失敗を隠して複雑化させている
状態: 未着手 / 元: playback-garbage-memo.md

`PlaybackController`は出力デバイス/ドライバ由来のランタイムエラーを検知すると、最大3回（`OUTPUT_STREAM_RECOVERY_MAX_ATTEMPTS`, `controller.rs:41`）まで裏でstop→reload→再生を自動試行する（`handle_output_stream_error`〜`run_output_stream_recovery_attempt`, `controller.rs:2292-2470`、リトライ間隔は`output_stream_recovery_delay`が100/500/1500msを返す）。この間、状態遷移とリロードが利用者に見えない形で繰り返される。

対応方針メモ: 音声出力層は「エラーを隠して裏で複雑な状態遷移を増やす」より「fail-fastで一度で確定的にエラーを見せる」方針の方が調査・保守コストが低いのではという論点がある。3回・100/500/1500msという値も経験則であり根拠が明文化されていない。リトライを残すか、初回失敗でエラー確定に倒すかは方針決定が先。

### Raw stream失敗時に非Rawへ暗黙にフォールバックする
状態: 未着手 / 元: playback-garbage-memo.md

`stream_mode == Raw`でのロード時、raw stream requestが失敗すると（出力デバイスエラーの場合を除き）ユーザーへの通知なしに"standard" stream request（transcoding指定なし・デフォルトbitrate/format・`raw`パラメータなしの通常リクエスト。Transcodingモード自体とは別経路）へ自動的に切り替わる（`controller.rs:1718-1777`）。ユーザーがRawを明示的に選択していても、失敗時に別経路へ黙って落ちるため、実際にどちらの経路で再生されたかがログ以外から分からない。

対応方針メモ: Raw失敗時はRawとして失敗を返す方が、選択と挙動の対応が明確になる。フォールバックを撤去するか、発生時にUI/ステータスへ明示するかの方針決定が必要。

---

## Tier 4

### QueueSyncGatewayがproduction no-opのままcontrollerに入り込んでいる
状態: 未着手 / 元: adr/20 #4

`create_playback_controller()`は全platformで`NoopQueueSyncGateway`を渡すが、controllerは`queue_sync` fieldを持ち複数箇所で`sync_queue_state()`を呼ぶ。テストもこのno-op seamを前提にしている。まだ存在しないConnect queue sync機能を先取りした抽象。

対応方針メモ: 実害は今のところない。Connect queue sync機能を実際に作る段になるまで放置してよい。

### Android authored codeが`src-tauri/gen/android`に埋まっている
状態: 未着手 / 元: adr/20 #9

Android実装の主要部分（Kotlin）が生成物ツリー配下にあり、生成されたproject treeと手書きruntime codeの境界が弱い。Tauri Androidの制約による部分あり。

対応方針メモ: 機能に影響しない構造上の見通しの悪さ。優先度は低い。

### global CSSにcomponent固有stateが残っている
状態: 未着手 / 元: adr/20 #10

`src/styles.css`のbase layerに`.song-item`系のcurrent/selected state相当のスタイルが残っている。component単位で完結していない。

対応方針メモ: 見た目の仕様変更が入るタイミングで一緒に手を入れるのが効率的。単独では優先度低い。

追補(ins):実は結構根深い問題と思われる。state管理としてのcomponentスタイリングを宣言しているものの、実情としてglobal cssファイルの過剰な一元管理と適当なmodulecss利用により宣言自体の清さと実情の汚さのギャップが激しい。state/affordanceなスタイル管理自体はtransonicにふさわしいが、実装や配備の方法は現状考慮が浅い状態と言わざるを得ない。要するに、この問題の裏にはtransonic全体のstate<->designの対応をどういうものにするかという方針の決定の必要性が隠れている。

### 意味が弱いテストと重複fixture
状態: 未着手 / 元: adr/20 小ゴミ

`ConnectButton.test.tsx`等に「落ちないことだけを固定する」テストがある。複数FEテストが`PlaybackStatus`・`ConnectSharedPlaybackState`・settings fixtureをそれぞれ手で作っている。

対応方針メモ: Tier 2の「settings正本分裂」「remote/local偽装」の解消と連動して自然に減る部分がある。単独で手を付ける優先度は低い。
→ 意味が弱いテストは表側には出ないもののかなり気持ち悪いので、速めに修正の時間を取りたい。また、ちゃんと方向性を明言しないと今後もダメダメなテストが量産されうるので、確定的な規約を熟考の上でしっかり作ってエージェント指示mdに記載しておきたいところ。(ins)

---

## 別件メモ（今回の整理中に見つかった、負債とは性質が異なるもの）

（現時点で未対応の別件メモなし）

## 解決済み

### Connect shared playbackのqueue意味論の型を統合（2026-07-04 / 元: adr/20 #8, Tier 1）

型（shape）ドリフトをRust権威のコード生成で根絶。振る舞い（reducer）はLocal/Connectで**意図的に分離のまま**とし、共有すべき不変条件だけを明文化した。

- Rustに権威型 `ConnectPlaybackCommand`（`models/connect.rs`）を追加し、`commands/playback.rs`・`connect.rs` の ad-hoc `serde_json::json!` 構築を全てそれ経由に統一。
- `ConnectPlaybackState`/`ConnectPlaybackCommand`/`PlayingState` を `#[typeshare]` 注釈し、`typeshare` + `typeshare.toml` で Go 構造体を `server/internal/realtime/connect_types_gen.go` へ生成。`pnpm go-types:export` / `go-types:check`（バージョン行を正規化して比較するドリフトチェック）を追加。要 `cargo install typeshare-cli --features go`。
- Go の手写し `playbackStateDoc`/`playbackCommandPayload` を生成型へ差し替え（reducer アルゴリズムは不変）。
- 唯一の共有不変条件（`playNextQueueLen` の意味 = `[currentIndex+1, +len)`）を `docs/playback-queue-semantics.md` に明文化し、Go/Rust 両 reducer からコメントで参照。
- スコープ外（別タスク）: `ConnectQueueResolveDialog` の push/pull、push/pull 用ワイヤ op 追加。

### settingsの正本をRustに一本化（2026-07-05 / 元: adr/20 #2, Tier 2）

TS側の default/normalize/legacy-migration の重複実装を撤去し、Rust を唯一の正本にした。

- `src/features/settings/service.ts` から `normalizeSettings`/`normalizePlaybackSettings`/`normalizeConnectSettings`/`normalizeAppearanceSettings` と内部 `splitUrlHostAndPort`・各 `normalize*` ヘルパーを削除。TS は正規化・移行を一切行わない。
- 旧 localStorage (`transonic.settings.v1`) の移行は、生JSONをそのまま Rust の `settings_update` に渡して正規化+移行+永続化させる方式に変更（`readRawLegacySettings`）。Rust 側 (`models/settings.rs` の custom `Deserialize` = `prebufferStrategy`→gapless / `serverUrl`→host+port 等) が既に移行を担っており、TS の二重実装は不要。
- `hydrateSettings` は Rust 由来の settings をそのまま信頼（再normalizeを撤去）。`setConnectSettings` の送信前 normalize も撤去（Rust が `result.data` で正規化済みを返す）。optimistic update + rollback は UI 側の関心として維持。
- driftしていた TS 側 default `gaplessPlaybackEnabled: false` を Rust default (`true`) に合わせて修正（プレースホルダとして残す `DEFAULT_SETTINGS` は「hydration前のみ」とコメント明記）。
- TS側の正規化を固定していた `service.test.ts` の3テスト（metered/output-device/legacy-migration）を、新しい振る舞い（Rust出力の verbatim 適用 / legacy blob の Rust 転送）のテストに置き換え。Rust 側は `app_settings.rs` に同等の移行テストが既存。

### pnpm package 整理のクローズと誤削除の是正（2026-07-05 / 元: adr/20 小ゴミ）

2026/7/4 の削除可否を検証。結果、大半は安全だったが `@testing-library/jest-dom` の削除は誤りだった。

- `@testing-library/jest-dom` は「未使用」ではなく `vite-plugin-solid` のテスト統合が `test.setupFiles` に自動注入する暗黙の必須依存。削除により pnpm strict linking 下でルートから解決できず「Cannot find module '@testing-library/jest-dom/vitest'」で **全 vitest suite がロード不能**になっていた。→ `devDependencies` に `@testing-library/jest-dom ^6.9.1` を復活。
- `vitest.shims.d.ts`（`/// <reference types="@vitest/browser-playwright" />`）は削除済みパッケージを参照し `tsc` を壊していた。browser モードは未使用のため file ごと削除。
- 残り（`@vitest/coverage-v8`・`playwright`・`@vitest/browser-playwright`・JS側 `@tauri-apps/plugin-opener`）の削除は問題なし。
- 併せて、queue意味論統合コミット (`da639e1`) 由来の既存 tsc break（`playbackDevice.test.ts` の fixture に `playNextQueueLen` 欠落）を修正。`pnpm test`(70) と `tsc --noEmit` はグリーン。

### roadmap-mermaid.md を削除（2026-07-05）

2026-03-23 時点の Phase 1〜4 計画で、記載内容は現状すべて実装済み。乖離したゴミとして `docs/roadmap-mermaid.md` を削除。
