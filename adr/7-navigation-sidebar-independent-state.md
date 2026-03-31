<!--
author: Codex
date: 2026-03-31 09:21
version: 0.0.1
-->

## 概要

- `NavigationSideBar` の責務と route の責務を分ける設計判断を行った。
- 現在の sidebar は `location.pathname` を読んで mode と選択状態を決め、mode 切替時にも navigate している。
- この構成では sidebar が本文の route に引きずられて更新され、常駐 UI としての振る舞いが不安定になることが分かった。

## 現状で確認したこと

### HomeLayout はもともと sidebar と本文を分離して配置している

- `src/routes/homeLayout.tsx` では `NavigationSideBar` と `props.children` を横並びに置いている。
- レイアウト上は、sidebar と本文は別の領域として扱える状態にある。

### NavigationSideBar が route に強く結びついている

- `src/components/navigation/NavigationSideBar.tsx` では `resolveBrowseMode(location.pathname)` で mode を決めている。
- folder browse では path から `libraryId` と `nodeId` を読み直している。
- item の選択状態も path から逆算している。
- `createEffect` は `location.pathname` に反応して一覧の再取得を行う。
- select 切替時にも `navigateToMode` で本文 route を変更している。

### browse 用 route にも sidebar の都合が混ざっている

- `src/routes/browse/folders.tsx` は本文そのものというより、library 数を見て redirect するための中継ページになっている。
- これは sidebar 側の mode 遷移や library 解決と本文 route の都合が混ざっているために生じている。

## 今回決めたこと

### NavigationSideBar は browse index へのアクセスだけを提供する

- sidebar は `NavigationSelect` で index 種別を選ばせる。
- sidebar は選択された種別に対応する index 一覧を表示する。
- sidebar は一覧 item を押したときだけ navigate を行う。
- select を切り替えただけでは本文を切り替えない。

### sidebar の状態は route から決めない

- `NavigationSelect` の mode は route から逆算しない。
- 本文が artist page であっても、sidebar が `Folder Structures` のまま残ることを許容する。
- これは、本文への遷移が「単一ページを見たい」という意図であり、sidebar の閲覧モード変更とは別の操作だからである。

### 復元するのは sidebar mode のみとする

- アプリ再起動時に復元対象とするのは `NavigationSelect` の mode のみとする。
- URL は復元対象に含めない。
- 起動後の本文は `/home` を基準にし、sidebar には前回の mode を適用する。

### back/forward は本文だけに作用する

- browser history の back/forward で変わるのは本文 route のみとする。
- sidebar はその操作に追従して mode や選択状態を変えない。

## この判断で整理できること

- `NavigationSideBar` から `pathname` を読んで mode を逆算する責務を外せる。
- `resolveBrowseMode`、route 由来の `libraryId` / `nodeId` 解決、mode 切替時の `navigateToMode` のような処理は不要になりやすい。
- sidebar の再読み込み契機を route change から切り離せるため、常駐 UI としての安定性が上がる。
- `props.children` 側は「現在開いている本文」、sidebar 側は「次に開く browse index」になり、役割が明確になる。

## 所感

- 現在の sidebar は route から状態を逆算する前提が強く、常駐ナビゲーションとしては責務が混ざっていた。
- transonic はデスクトップアプリであり、sidebar の閲覧モード記憶を URL より優先して扱う方が実際の操作感に合っている。
- 本文 route と sidebar state を相互に矯正しない設計の方が、今回の browse UI では単純で読みやすい。
