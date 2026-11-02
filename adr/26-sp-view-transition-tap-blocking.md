<!--
author:   Claude
date:     2026-11-02 23:00
version:  0.0.1
-->

## SP の画面遷移中にボトムバーのタップが効かない原因と、遷移を残したままの解消法

### 症状

~~SP で、SPExpandablePlayerBar が表示されている（再生中の）状態でボトムバー（`SPBottomNavigation`）を続けてタップすると、2回目以降のタップが反応しないことがあった。~~\
(2026-11-03 00:40) 実機で報告されていた「シートの開閉直後にボトムバーのタップが効かない」という症状の原因は、この ADR の View Transition ではなく、シート上のスワイプがフリック扱いされて直後のタップが抑制されることだった（`adr/27`）。この ADR に記録した問題は、画面遷移中（View Transition の実行中）にボトムバーをタップすると、そのタップが捨てられるというもので、ハーネスで画面遷移の直後に続けてタップして再現した。

### 原因

`useSPNavigate`（`src/features/navigation/useSPNavigate.ts`）は SP の画面遷移を `document.startViewTransition()` で包んでフェードさせている。View Transition の実行中は、ブラウザがビューポート全体を覆う `::view-transition` 擬似要素をトップレイヤーに置くため、その間のタップはすべて `<html>` に当たり、`NavigationItem` の `onClick` には届かない。

ハーネスで HEAD の `SPHomeLayout` を描画し、ボトムバーを続けてタップして確認した。1回目の `click` は `<i>`（アイコン）に届いて遷移が始まり、遷移中の2回目の `click` は対象が `HTML` になって遷移が起きなかった。遷移を構成する擬似要素のアニメーションは各 250ms（`sp-screen` は 180ms）で、無限ループのものは無かった。

これはイベント処理の優先順位の問題ではなく、ヒットテストの結果である。遷移用の擬似要素がタップ位置の最前面と判定され、擬似要素はイベントの対象になれないためイベントは `<html>` に送られる。下にある実際の要素にはイベントがそもそも届かない。

ヒットテストで最前面になり、タップが下の要素に届かなくなる原因は3つある。

1. `::view-transition` 擬似要素（ビューポート全体を覆う）。
2. `:root` に既定で付く `view-transition-name: root` によるページ全体のスナップショット。
3. 自身に `view-transition-name` を持つ要素。遷移中は実物が描画されず、代わりに表示される擬似要素が最前面になるため、タップは実物の要素に届かない（作者側で打ち消せない）。このプロジェクトではボトムバー自身が `view-transition-name: sp-bottom-navigation` を持っている。

### 遷移を残したままの解消法

これは View Transitions で広く知られた問題で、定番の対処は次の2行である。

```css
:root { view-transition-name: none; }
::view-transition { pointer-events: none; }
```

ただし、このプロジェクトでは上記3に当たるボトムバーの `view-transition-name` も外さないとタップは通らない。名前を外したボトムバーは遷移中も実物のまま描画され、タップを受け付ける。

最初の試行では `::view-transition { pointer-events: none; }` だけを足し、`:root` とボトムバーの名前を残していたため効果が無く、「CSS だけでは解消できない」と誤って判断しかけた。3つのうち1つだけを外しても解消しない点に注意が必要である。

この対処の副作用として想定したものは次のとおり。

- ボトムバーの上向きの影が、遷移中は `sp-screen` のスナップショットの下に隠れる可能性がある。
- `root` のクロスフェードが無くなる。`root` に残るのはページ間で変わらない背景などなので、見た目への影響はほぼ無いと見込んだ。
- 遷移開始直後、新しい状態を撮影するまでの描画停止中の入力は、仕様上どの設定でも無視される（CSSWG #7797）。

遷移中のタップを新しい側の要素へ通す仕組み（`pointer-events` の拡張など）は CSSWG で議論されているが（#9586、#10930）、2025年8月時点では結論が出ていなかった。

### 参考

- [View Transitions: Careful Not To Make Stuff Unclickable](https://blog.master.dev/view-transitions-careful-not-to-make-stuff-unclickable/)
- [csswg-drafts #11596: The ability for authors to re-enable pointer events](https://lists.w3.org/Archives/Public/public-css-archive/2025Jan/0810.html)
- [csswg-drafts #7797: User input should be ignored when rendering is suppressed](https://lists.w3.org/Archives/Public/public-css-archive/2023Mar/0623.html)
- [csswg-drafts #9586: `:hover` doesn't work during a transition](https://lists.w3.org/Archives/Public/public-css-archive/2023Nov/0289.html)
