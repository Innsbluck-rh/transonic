<!--
author:  Claude
date:    2026-04-05 14:05
version: 0.0.1
-->

## CJKフォントのフォールバックをOSシステムフォントに変更した

- 音楽プレイヤーという性質上、アーティスト名や曲名にあらゆる言語の文字が混在する可能性がある。
- Archivo / Public Sans はラテン文字フォントであるため、CJK文字はOSのフォールバックに委ねられる。
- Windowsでは、CJKのデフォルトフォールバックがSimSun（宋体）やMingLiU（細明體）等のセリフ体になる場合があり、これらはBoldウェイトを持たない。
- その結果、たとえば「梁翹柏」の「翹」のような文字にfont-weight: 900を指定しても明朝体のまま描画され、UIから浮いて見える問題が発生していた。
- AndroidではシステムにNoto Sans CJKが組み込まれているため、この問題は発生しなかった。

## Noto Sans CJK webfontバンドルの問題

- 対策としてNoto Sans JP / KR / SC / TCをGoogle Fonts経由で`vite-plugin-webfont-dl`によりバンドルする方法を試した。
- dev環境での表示は改善されたが、CJKフォントは全ウェイトで数十MB〜100MB超になるため、ビルド時間が極端に増加した。
- また、AndroidではシステムにNoto Sans CJKがすでに存在するため、同じフォントを二重にダウンロードすることになり無駄が大きかった。

## OSシステムフォントへのフォールバックを採用した

- Tauriデスクトップアプリであるためターゲットプラットフォームが既知であり、各OSに搭載されているフォントを前提にできると判断した。
- Windows 10以降には以下のサンセリフCJKフォントが搭載されており、いずれもBold（700）ウェイトを持っている。

  | フォント名 | 対象言語 | 利用可能ウェイト |
  |---|---|---|
  | Microsoft YaHei（微软雅黑） | 簡体字中国語 | Light / Regular / Bold |
  | Microsoft JhengHei（微軟正黑體） | 繁体字中国語 | Light / Regular / Bold |
  | Yu Gothic（游ゴシック） | 日本語 | Light / Regular / Medium / Bold |
  | Malgun Gothic（맑은 고딕） | 韓国語 | Semilight / Regular / Bold |

- weight: 900が指定されている箇所では、Bold（700）からブラウザがfaux bold（合成太字）を生成する。セリフ体の細い字形から合成されるよりも遥かに自然な見た目になる。
- Androidではシステムの`sans-serif`がNoto Sans CJKに解決されるため、追加の指定は不要である。
- macOSではHiragino Sans、PingFang SC/TC等が搭載されており、`sans-serif`が適切にCJKフォントへ解決されるため同様に追加指定は不要である。
- Linuxについてはディストリビューションにより状況が異なるが、CJK楽曲を扱うユーザーはCJKフォントを導入済みである可能性が高く、`sans-serif`のフォールバックで十分と判断した。

## font-familyの指定順序について

- SC → TC → JP → KR の順で指定した。
- Unicodeの漢字統合（Han Unification）により同一コードポイントで字形が異なるケースが存在する（例: 日本語の「直」と簡体字の「直」など）。
- 音楽プレイヤーでのアーティスト名・曲名表示という用途では、字形の微細な差異よりも文字が太字サンセリフで統一的に描画されることの方が重要であると判断した。

## CJK以外のスクリプトについて

- タイ語、アラビア語、ヒンディー語等については、Windowsのフォントリンク機構によりサンセリフ系フォント（Segoe UI, Nirmala UI, Leelawadee UI等）に解決されるため、明示指定は不要と判断した。
- ヒエログリフのような極めて特殊な文字体系については、OSに対応フォントが存在しない場合もあるが、バンドルサイズへの影響を考慮し対応しないこととした。

## 変更内容

- `vite.config.ts` からNoto Sans CJKのwebfontダウンロード指定を削除した。
- `src/styles.css` の各`font-family`定義で、Noto Sans JP / KR / SC / TCをWindowsシステムフォント（Microsoft YaHei, Microsoft JhengHei, Yu Gothic, Malgun Gothic）に置き換えた。
- `h1, h2, h3`のルールにもCJKフォールバックが欠落していたため追加した。
