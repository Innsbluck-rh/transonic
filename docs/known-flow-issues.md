# 検証・生成フロー負債バックログ

## 位置づけ

- `docs/known-issues.md` が**プロダクトコードの実装負債**を扱うのに対し、このファイルは**検証・生成フローそのもの**（test / check / コード生成 / 整形 / ビルド前後フック）の負債を扱う。
- `docs/known-issues.md` と同じく「生きた」バックログ。優先度・状態・対応方針を随時書き換えてよい。対応が終わった項目は削除せず末尾の「解決済み」へ日付付きで移動する。
- ADRとは役割が異なる（ADRは過去の意思決定の記録であり、今後の予定を書かない）。
- 初版はClaude Codeによる 2026-07-27 の実測調査に基づく。以降、コントリビュータの追記は `(ins)` を付ける。
- 以下の測定値・テスト数・構成の記述は、その調査時点の本体を対象とする。別worktreeの未統合変更は含まない。

## 判断の軸

このバックログは次の軸で項目を分けている。1・2はコントリビュータが示した方針そのもの、3は調査中に判明した前提の綻び。

1. **同等のチェック/生成ができるなら、速い方がよい** → Tier A。出力も検出能力も変わらないので、議論なしで着手してよい。
2. **速さと引き換えに形式・検出範囲・デバッグ体験が変わる** → Tier C。方針決定が先。
3. 加えて、調査中に**「そもそも同等のチェックになっていない」穴**が複数見つかったため Tier B として分離した。1の前提（＝同等であること）が崩れている箇所なので、速度より先に扱う価値がある。

---

## 見立て（結論）

- Rust を1ファイル編集した後の検証セット（`bindings:check` → `cargo:check` → `cargo:test`）は実測 **52.9s**（`bindings:export` を使う変種では 34.9s）。Rustテスト本体は0.56sで、所要時間の大部分はビルドに費やされている。コマンド切り替え時の再コンパイルも観測されたが、その発生原因は未確定。
- **Tier A-1 / A-2 は大きな短縮が期待できる候補**（検証ループから `cargo run`（bindings）を外し、bindingsドリフト検査をテストに移す）。同じ1編集に対し `cargo check` → `cargo test --workspace` は実測 **17.5s**。ただし、この手順はB-2の13テストを含む一方、bindingsドリフト検査を含まない。**同等性を保つには検査の移行が必要で、移行後の所要時間は未計測。**
- **Tier B は Tier A より先に扱う価値がある。** 「同等のチェックができるなら速い方がよい」の前提である「同等」が現状崩れている（テストコードを見ていない `cargo:check`、対象外の13テスト、どの自動経路にも無い `tsc`）。B-2・B-3の実行時間の増分は小さい（`--workspace` は所要時間ほぼ不変、`tsc` は2.4s）。
- **Tier C は A・B を入れてから再計測して判断すべき。** debuginfo削減（-25〜30%）もリンカ変更も、A-1/A-2で往復自体が消えれば効く場面が減る。現時点で先行導入する理由は薄い。
- `cargo-nextest` は**不要**（Rustテスト実行は既に 0.56s / 228テスト。ボトルネックは実行ではなくビルド）。
- vitest / Go は現状おおむね健全。vitest の伸びしろは約0.8sでリスクの方が大きく、Go はビルドキャッシュが良く効いている（キャッシュ時 0.6s）。

---

## 測定環境と方法

- Windows 11 Home 10.0.22631 / rustc 1.91.1 / cargo 1.91.1 / Node 22.14.0 / pnpm 11.5.2 / Go 1.26.2（`server/go.mod` は `go 1.23`）
- `src-tauri/target` はウォーム状態から測定。ネットワークI/Oを伴う処理は含まない。
- 「Rust 1編集後」は `src-tauri/src/playback/controller.rs` の mtime を更新して再ビルドを誘発した値。ソース内容の変更は伴わない。
- 各コマンドは1〜3回連続実行した代表値。**±20%程度のぶれがある**。単発で 55s に跳ねる外れ値も観測したが、原因は未確認。以下の数値は絶対値ではなく**相対比較のための目安**として読むこと。

## 実測サマリ

| コマンド | 変更なし | Rust 1編集後 | 備考 |
| --- | --- | --- | --- |
| `cargo check`（= `pnpm cargo:check`） | 3.3s | 3.2s | codegen/link をしないので安い |
| `cargo test`（= `pnpm cargo:test`） | 12.2s | 15〜17s | ほぼ全部ビルド時間 |
| `cargo run --bin export-bindings`（= `bindings:export`） | 1.0s | 12〜15s | |
| `cargo run --bin check-bindings`（= `bindings:check`） | 1.7s | 12〜15s | |
| `vitest run` | 6.6〜7.9s | — | |
| `tsc --noEmit` | 2.4s（直接） / 3.3s（`pnpm exec`） | — | スクリプト未定義 |
| `prettier --check .` | 7.6s | — | `--cache` ウォームで 4.0s |
| `typeshare` + `gofmt` + `git diff`（= `go-types:check`） | 1.3s | — | |
| `go test ./...`（`server/`） | 0.6s（キャッシュ済） | — | Goのビルドキャッシュは良好に効いている |

**テスト本体の実行時間はごく短い。**

- Rust: 228 tests / **0.56s**（`src-tauri` 本体）
- Rust: 13 tests（`crates/opensubsonic-client`、後述のとおり現行フローでは走っていない）
- フロントエンド: 18 files / 75 tests、`tests 1.02s`（vitest 内訳）
- Go: 全パッケージ キャッシュ時 0.6s

つまり**現状のフロー時間はほぼ全てビルド/環境生成であり、テストの中身ではない**。最適化の対象は一貫してビルド側にある。

---

## 大きな時間コスト: コマンド切り替え時の `transonic` crate の再コンパイル

調査時の環境では、`cargo test` と `cargo run --bin` / `cargo build --bin`（bindings生成系）を交互に実行すると、`transonic` crate 本体の再コンパイルが観測された。以下ではこの切り替えを「往復」と呼ぶ。`test` プロファイルは既定で `dev` を継承するため、プロファイル名の違いだけでは原因を説明できない。ビルド設定や成果物の再利用条件は未調査。

```
$ cargo test                      # ...
$ cargo build --bin export-bindings
   Compiling transonic v0.0.1
    Finished `dev` profile ... in 11.99s
$ cargo test --no-run
   Compiling transonic v0.0.1
    Finished `test` profile ... in 10.60s
```

この測定では依存クレートは再ビルドされず、**`transonic` 本体の再コンパイルに 10〜12s かかった**。

一方 `cargo check` は対象コードのcodegen/linkをしない。調査時には、その前後で同様の再コンパイルは観測されなかった（`cargo check` 直後の `cargo run` は 0.9s）。

CLAUDE.md / AGENTS.md / `adr/18` にはbindings検査・Rust検査/テスト・FEテストが挙げられているが、実行順序を固定する単一スクリプトはない。以下の測定では `bindings:check` → `cargo:check` → `cargo:test` の順序を使用した。

Rust 1ファイル編集後の実測（連続実行）:

| 手順 | 所要 | 備考 |
| --- | --- | --- |
| `bindings:check` → `cargo:check` → `cargo:test` | **52.9s** | bindingsドリフト検査あり、Rustはルートパッケージのみ |
| `bindings:export` → `cargo check` → `cargo test --workspace` | 34.9s | bindingsを書き換える手順で、ドリフト検査とは異なる |
| `cargo check` → `cargo test --workspace`（cargo run を外しただけ） | **17.5s** | workspaceのテストを含むが、bindingsドリフト検査はない |

`cargo run` を含まない手順は含む手順より短かった。ただし検査内容が異なるため、**同等の検査を維持した場合の短縮幅はA-2の実装後に確認する必要がある。**

---

## Tier A: 同等性を保ったまま速くできる（方針①・議論不要）

### A-1. 検証ループから `cargo run`（bindings）を外す
状態: 未着手

`bindings:check` は「Rustの型からTSを生成し、チェックイン済みの `src/bindings.ts` と比較する」ためだけに `dev` プロファイルのバイナリをビルド・リンクして実行している。これが前述の往復の片側を作っている。

対応方針メモ: A-2でbindings検査を維持した上で、検証時に走らせるcargoコマンドを `cargo test` 系に寄せ、再コンパイルの削減を確認する。`cargo check`（3.2s）は速い一次シグナルとして残してよい。`bindings:export` は `predev` 由来の書き込み系なので検証ループとは別扱いでよい。

### A-2. bindingsドリフト検査を `#[test]` に移す
状態: 未着手 / A-1の具体案

`check_typescript_bindings()` は既に `transonic_lib::bindings` のpub関数であり、同じ生成結果と `src/bindings.ts` の比較をテストから呼ぶ案。ただし `lib.rs` は `bindings` モジュールを `#[cfg(not(test))]` でテストから除外しているため、単に `#[test]` を追加するだけでは実行されない。モジュール構成の変更またはintegration test等の別経路が必要で、ビルド時間・検査の同等性は実装後に確認する。

対応方針メモ: `check-bindings` bin は `prebuild` が使っているため、bin自体を消すかどうかは別途判断が要る。テスト化だけ先に入れてbinを残す形も可能。`export-bindings` bin は書き込み用途なので維持。

### A-3. `prettier --check` / `--write` に `--cache` を付ける
状態: 未着手

実測 7.6s → **4.0s**（ウォーム）。ただしプラグインのバージョン・実装変更はキャッシュキーに含まれないため、本プロジェクトの整形プラグイン更新時にはキャッシュ削除が必要。既定の保存先は `node_modules/.cache/prettier/`。既存のignore対象外へ保存する場合は `.gitignore` に追加する。

---

## Tier B: 「同等のチェック」になっていない穴（速度以前の問題）

### B-1. `pnpm cargo:check` は `#[cfg(test)]` を検査していない
状態: 未着手

`cargo check` は `--all-targets` なしではテストコードをコンパイル対象にしない。実証: `playback_state.rs` の `#[cfg(test)]` モジュールに型エラーを仕込むと、

- `cargo check` → **exit 0（素通り）**
- `cargo check --all-targets` → `error[E0308]: mismatched types` を検出

`cargo:check` だけを通して「検証した」と判断すると壊れたテストコードを見逃す。

対応方針メモ: `--all-targets` を付けると 3.1s → 14.3s に増える（テストターゲットのコンパイルが増えるため）。`cargo test` を通すなら重複するので、「速い一次シグナルとしての `cargo:check`（現状のまま）」と「ゲートとしての `cargo:test`」の役割分担を明示するのが素直か。この役割分担は AGENTS.md に明記した（2026-10-11）。

### B-5. CIが存在しない
状態: 未着手

`.github/workflows` が無く、GitHub Actionsによる継続的な検証はない。ローカルにはpre-commitの整形、`prebuild` のbindings検査、`vite:build` の型検査がある。検証セットは `pnpm verify` で実行できるが、GitHub上で自動実行する経路はない。CLAUDE.md の「CI/buildを落とす」という説明に対応するCI設定はリポジトリ内に存在しない。

対応方針メモ: B-1〜B-4を是正した検証をCIで実行すれば、実行漏れを減らせる。ただしWindows前提のビルド時間が長いこと、Android/ASIO など環境依存が強いことから、どこまでを自動化するかは方針決定が要る。まずは安いもの（`tsc`・`vitest`・`go test`・`go-types:check`）だけでも意味がある。

---

## Tier C: 速さと引き換えに何かが変わる（方針②・要議論）

### C-1. `[profile.dev] debug` の引き下げ
状態: 未着手 / 要方針決定

`transonic` crate の再コンパイル（前述の10〜12s）は debuginfo 生成が相当量を占める。実測:

| `[profile.dev] debug` | test→run | run→test |
| --- | --- | --- |
| 既定（full） | 11.6s | 10.3s |
| `line-tables-only` | 8.7s | 7.6s |
| `0` | 8.1s | 6.8s |

**トレードオフ**: `line-tables-only` はバックトレースの行番号を保つが、デバッガでの変数インスペクションが効かなくなる。`0` は行番号も失う。設定はプロファイル全体に効くため `pnpm dev` / `tauri dev` の Rust デバッグ体験にも影響する。また変更時に依存グラフ全体のフルリビルドが一度発生する（実測 約150s）。

対応方針メモ: 測定時の削減幅は約25〜30%。Tier A-1/A-2で再コンパイルを減らせれば、この設定変更の効果も変わるため、**A-1/A-2 を先に入れてから再計測する**。なお `[profile.dev.package."*"] debug = 0` として依存クレートだけ下げる案は、日常の再コンパイル対象が `transonic` 本体である以上ほとんど効かない。

### C-2. vitest の `pool: 'threads'` + `isolate: false`
状態: 未着手 / 要方針決定

vitest の内訳は `Duration 6.63s (transform 5.61s, setup 5.29s, import 7.36s, tests 1.09s, environment 41.71s)`。`environment`（jsdom生成）が全ワーカー合計で圧倒的に大きく、テスト本体は 1.09s。

実測（全75テストが通ることは確認済み）:

| 設定 | Duration |
| --- | --- |
| 現状（既定 `forks` + `isolate: true`） | 6.63s |
| `threads` + `isolate: false` | 5.79s |

**トレードオフ**: `isolate: false` はテストファイル間でモジュール状態・グローバルを共有するため、テスト間汚染が起きうる。現状は全て通るが、将来テストが増えたときに順序依存のフレーキーさを持ち込むリスクがある。得られるのは約0.8s。

対応方針メモ: 費用対効果が薄く、リスクの方向が「テストの信頼性」なので、**現時点では見送りが妥当**という見立て。なお `docs/known-issues.md` の「意味が弱いテストと重複fixture」でテスト規約を作る方針が挙がっており、そこで隔離方針も一緒に決めるのが筋。

### C-3. jsdom → happy-dom
状態: 未計測 / 要方針決定

`environment` が支配的である以上、より軽量な happy-dom への置換は理屈上は効く。ただし未導入のため未計測。

**トレードオフ**: DOM実装が変わるため、既存テスト（`ConnectSettings` / `QueueList` / `VolumeSlider` など DOM を触る9ファイル）の挙動差が出る可能性がある。依存を1つ増やすことにもなる。

対応方針メモ: 得られる上限は vitest 全体の 6.6s のうちの一部でしかなく、Rust側の10〜12sに比べると小さい。**優先度は低い**。やるなら C-2 と合わせて「FEテスト実行環境の方針」としてまとめて決める。

### C-4. リンカを `rust-lld` に変更
状態: **計測済み・不採用と判断**

`cargo run`/`cargo test` が `cargo check`（3.2s）に対して10〜15s かかる差の大部分は codegen + リンクである。実測: 1編集後の `cargo check --bin export-bindings` = 3.1s に対し `cargo build --bin export-bindings` = 11.9s。**約9sがcodegen+リンク**であり、リンカ交換は理屈上もっとも効きそうな候補だった。

`rust-lld.exe` はツールチェーンに同梱されている（`<sysroot>/lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe`）。`RUSTFLAGS="-Clinker=<path>/rust-lld.exe -Clinker-flavor=lld-link"` で **stable の MSVC ターゲットでも問題なくビルドが通ることは確認できた**（フルビルド完走、exit 0）。

独立した target ディレクトリを2つ用意し、同条件で比較した結果:

| | フルビルド | 1編集後の再ビルド（3回） | 中央値 |
| --- | --- | --- | --- |
| `rust-lld` | 183.0s | 17.0s / 18.0s / 22.4s | 18.0s |
| 既定（MSVC `link.exe`） | 190.4s | 21.1s / 16.8s / 25.6s | 21.1s |

**再ビルド中央値の差は約15%、フルビルドの差は約4%。** 再ビルドは各3回でばらつきも大きく、この測定では安定した改善を確認できなかったため不採用とする。

対応方針メモ: この測定だけでは、約9sのcodegenとリンクの内訳は特定できない。リンカ交換の効果を再検討する場合は、各工程の時間を分けて確認する必要がある。

### C-5. `cargo-nextest`
状態: **不採用と判断**

テスト実行の並列化・高速化ツールだが、**本プロジェクトのRustテスト実行時間は 0.56s / 228テスト**であり、削るべき時間がそもそも無い。ボトルネックはビルドであって実行ではないため、導入しても体感は変わらない。ツールを1つ増やすコストに見合わない。

---

## Tier D: 環境・ディスク衛生

### D-1. `src-tauri/target` が58GBに膨張
状態: 未着手

| ディレクトリ | サイズ | 最終更新 |
| --- | --- | --- |
| `debug` | **42.0 GB** | 2026-07-27 |
| `codex-test-clean` | 5.8 GB | 2026-06-20 |
| `codex-test-run` | 5.8 GB | 2026-06-19 |
| `release` | 3.8 GB | 2026-07-26 |
| `aarch64-linux-android` | 1.0 GB | 2026-06-13 |
| `flycheck0` | ~0 GB | 2026-07-04 |

`codex-test-*`（11.6GB）は1か月以上更新されていない別ツール由来の残骸。`flycheck0` は rust-analyzer 由来。`debug` の42GBは、プロファイル/フラグを変えるたびに成果物が累積したもの。

対応方針メモ: 容量の増大が検証時間へ与える影響は未測定。未使用の成果物は削除対象になり得るが、利用中でないことを確認する。`debug` の掃除は再ビルド約150s以上のコストと引き換え。

### D-2. `opusic-sys` の cmake ビルドが長いパスで失敗する
状態: 記録のみ（実害は現状なし）

調査時、`CARGO_TARGET_DIR` を深いパス（例: 一時ディレクトリ配下）に置くと、`opusic-sys`（`symphonia-adapter-libopus` 経由）のcmakeビルドが `CMake Error at CMakeLists.txt:7 (project)` で失敗した。パス長制約が疑われるが、このエラー行だけでは原因を確定できない。

対応方針メモ: 通常の `src-tauri/target` では起きない。ただし「テスト用に別 target ディレクトリを切る」類の対策を採る場合はパス長に注意が要る。短いパス（例: `C:\<短い名前>`）なら通る。

---

## 付録: 検証したが不発だった仮説

記録しておかないと再度同じ調査をするため残す。

- **vitest から `tailwindcss` / `vite-plugin-webfont-dl` プラグインを外せば速くなる**（`vite.config.ts` に `test` ブロックが無く、テストがdevサーバ用のフル構成を読み込んでいるため疑った） → **効果なし**。実測 7.39s（プラグイン除去）vs 6.63s（現状）で誤差範囲。ボトルネックはプラグインではなく jsdom 環境生成。
- **`cargo check` と `cargo run` の切り替えでも同様に再コンパイルされる** → **調査時には観測されなかった**。`cargo check` 直後の `cargo run --bin export-bindings` は 0.9s。再コンパイルを観測したのは `cargo test` と `cargo run`/`cargo build` の切り替え。

## 解決済み

### B-4. Non-mutating Go type drift check (2026-10-11) (ins)

`go-types:check` now generates into a temporary directory, formats with `gofmt`, and compares against the working file without relying on Git state. CRLF and LF are treated equivalently. Temporary files are removed on success and failure.

Verified matching output, CRLF output, unstaged and staged drift, a correct working file with a stale index, missing generator and formatter commands, and generator failure. Each case preserved the working file and index and left no temporary files.

### A-4 / B-2 / B-3. 検証セットの統合・workspaceテスト・型検査（2026-10-11）(ins)

`pnpm verify` にTypeScript型検査、FEテスト、Go型ドリフト検査、Goテスト、Rustチェック、TS bindingsドリフト検査、Rust workspaceテストを順に組み込んだ。失敗時は後続の検証を実行しない。

`cargo:test` に `--workspace` を追加し、`opensubsonic-client` を対象に含めた。`typecheck`（`tsc --noEmit`）を追加し、`vite:build` の先頭でも実行するため、Tauriの `beforeBuildCommand` 経由でも型エラーを検出する。

Vitestの収集対象は `src/**/*.test.{ts,tsx}` に限定し、リポジトリ内の別worktreeやvendoredコードのテストを実行しない。

検証: `pnpm verify`（FE 75件、Rust 228+13件、Go全パッケージ、型検査・生成型検査）と `pnpm vite:build` が成功。型エラーを一時的に追加すると `typecheck`・`verify`・`vite:build` が非ゼロで終了し、後続処理を実行しないことも確認した。
