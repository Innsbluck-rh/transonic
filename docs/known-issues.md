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

### settingsの正本がTypeScriptとRustに分裂
状態: 未着手 / 元: adr/20 #2

default・normalize・legacy migration・optimistic update・rollbackが`src/features/settings/service.ts`と`src-tauri/src/models/settings.rs`等の両方にある。CLAUDE.mdで既に「Rust側を正本にする」という方針は明文化済み（`adr/20`本文にも同旨の記載あり）。つまりここは方針決定は済んでおり、実装への反映だけが残っている。

対応方針メモ: Tier 2の中では最も「やることが決まっている」項目。着手コストが見積もりやすい。

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

### pnpm packageの未使用・配置違い
状態: 着手済み(ins) / 元: adr/20 小ゴミ

`@testing-library/jest-dom`・`@vitest/browser-playwright`・`@vitest/coverage-v8`・`playwright`・`@tauri-apps/plugin-opener`は現状未使用。`jsdom`・`prettier-plugin-tailwindcss`は`devDependencies`が自然。

対応方針メモ: 機械的に削れる。手が空いたタイミングでいつでも着手可能。
→ 2026/7/4 着手済みのはず。別件の調査であれこれを見た者は軽くpackage.jsonをチェックしてから、消しても問題なさそうかの可否を報告すること。(ins)

### 意味が弱いテストと重複fixture
状態: 未着手 / 元: adr/20 小ゴミ

`ConnectButton.test.tsx`等に「落ちないことだけを固定する」テストがある。複数FEテストが`PlaybackStatus`・`ConnectSharedPlaybackState`・settings fixtureをそれぞれ手で作っている。

対応方針メモ: Tier 2の「settings正本分裂」「remote/local偽装」の解消と連動して自然に減る部分がある。単独で手を付ける優先度は低い。
→ 意味が弱いテストは表側には出ないもののかなり気持ち悪いので、速めに修正の時間を取りたい。また、ちゃんと方向性を明言しないと今後もダメダメなテストが量産されうるので、確定的な規約を熟考の上でしっかり作ってエージェント指示mdに記載しておきたいところ。(ins)

---

## 別件メモ（今回の整理中に見つかった、負債とは性質が異なるもの）

- `docs/roadmap-mermaid.md`は2026-03-23時点のPhase計画で、記載されているPhase 1〜4は現状すべて実装済み。内容が現状と乖離しているため、要不要を判断して整理する余地がある（本ファイルの対象外として扱う）。
→ これ、最初期にとりあえずで作ったゴミに近い...いや、ゴミであると思われる。消してよければタイミングで消すべき。(ins)

## 解決済み

### Connect shared playbackのqueue意味論の型を統合（2026-07-04 / 元: adr/20 #8, Tier 1）

型（shape）ドリフトをRust権威のコード生成で根絶。振る舞い（reducer）はLocal/Connectで**意図的に分離のまま**とし、共有すべき不変条件だけを明文化した。

- Rustに権威型 `ConnectPlaybackCommand`（`models/connect.rs`）を追加し、`commands/playback.rs`・`connect.rs` の ad-hoc `serde_json::json!` 構築を全てそれ経由に統一。
- `ConnectPlaybackState`/`ConnectPlaybackCommand`/`PlayingState` を `#[typeshare]` 注釈し、`typeshare` + `typeshare.toml` で Go 構造体を `server/internal/realtime/connect_types_gen.go` へ生成。`pnpm go-types:export` / `go-types:check`（バージョン行を正規化して比較するドリフトチェック）を追加。要 `cargo install typeshare-cli --features go`。
- Go の手写し `playbackStateDoc`/`playbackCommandPayload` を生成型へ差し替え（reducer アルゴリズムは不変）。
- 唯一の共有不変条件（`playNextQueueLen` の意味 = `[currentIndex+1, +len)`）を `docs/playback-queue-semantics.md` に明文化し、Go/Rust 両 reducer からコメントで参照。
- スコープ外（別タスク）: `ConnectQueueResolveDialog` の push/pull、push/pull 用ワイヤ op 追加。
