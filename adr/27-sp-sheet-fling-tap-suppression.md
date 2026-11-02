<!--
author:   Claude
date:     2026-11-03 00:30
version:  0.0.1
-->

## SP のシート開閉直後にボトムバーのタップが効かない原因（フリック後のタップ抑制）

### 症状

Android 実機で、SPExpandablePlayerBar のシートをスワイプで開閉した直後にボトムバー（`SPBottomNavigation`）をタップすると、1回のタップでも反応しないことがあった。

`adr/26` ではこの症状の原因を View Transition 中のヒットテストだと考えて記録したが、それは別の問題で、この症状の原因ではなかった。View Transition の件の修正を入れた後も、この症状は実機で残っていた。

### 調べ方

デスクトップの Chromium で実コンポーネントを描画したハーネスでは、マウスのクリックを使う限り再現しなかった。そこで `pnpm ad:dev` のデバッグビルドを実機に入れ、`adb forward tcp:<port> localabstract:webview_devtools_remote_<pid>` で WebView の DevTools に接続した。Chrome DevTools Protocol の `Runtime.evaluate` で、`touchstart` / `touchend` / `pointerdown` / `pointerup` / `pointercancel` / `click` などの届き先を記録する処理を document の capture フェーズに仕込み、実機で操作して記録を読んだ。

### 原因

記録では、失敗したタップは `pointerdown`・`pointerup`・`touchend` までボトムバーの項目に届いており、`click` だけが発火していなかった。失敗したタップはどれも、シート上の素早いスワイプの直後（指を離してから約 30〜50ms 以内）に始まっていた。間を空けたタップには `click` が出ていた。

これは Chromium のタップ抑制（`TapSuppressionController`）によるものである。

1. シート上の素早いスワイプを、ブラウザはフリック（慣性スクロール）として扱う。シートのドラッグ領域には `touch-action: none` が付いているが、それでもフリックとして扱われていた。
2. フリックの最中に画面に触れると、そのタップは「フリックを止めるためのタップ」とみなされ、`click` が発火しない。

フリックの最中にタップした指がわずかに動き、ボトムバー側で `pointercancel` になるケースもあった。

シートの開閉アニメーション自体は原因ではなく、直前のスワイプがフリック扱いされていたことが原因だった。

### 修正

`SPExpandablePlayerBar`（`src/components/sp/player/SPExpandablePlayerBar.tsx`）のルート要素に、`passive: false` で `touchmove` のリスナーを登録し、次の場合に `preventDefault()` するようにした。

- シートをドラッグしている間（`dragPointerId` がある間）
- タッチの対象がシート内のスクロール可能な要素（キューのリスト）の中ではない場合

`touchmove` を打ち消すとブラウザはスクロールの動作を始めないため、フリックが発生せず、直後のタップも抑制されない。シートのドラッグは pointer イベントで処理しているので影響は無い。キューのリストのスクロールは打ち消さないので、従来どおりスクロールできる。

最初は「ドラッグ中だけ打ち消す」形にしたが、それでは一部のタップが失敗し続けた。展開したプレイヤーの曲名・アーティスト名・操作ボタンは `pointerdown` の伝播を止めているため、そこから始めたスワイプはシートのドラッグにならず、ドラッグ中の判定に入らなかったためである。判定を「スクロール可能な要素の外なら常に打ち消す」に広げた後は、実機の記録で、開閉直後のボトムバーのタップがすべて `click` まで届き、遷移することを確認した。

### 補足

- マウスのクリックでは再現しないため、この種の問題はデスクトップのハーネスでは確認できない。実機の WebView に DevTools で接続して、イベントの届き先を記録するのが有効だった。
- `touch-action: none` はスクロールを止めても、フリックとしての扱いとそれに続くタップ抑制までは止めなかった。

### 参考

- [Chromium tap_suppression_controller.h](https://chromium.googlesource.com/chromium/src.git/+/HEAD/components/input/tap_suppression_controller.h)
