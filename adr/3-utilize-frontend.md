<!--
author: innsbluck
date: 2026-03-24 16:03
version: 0.0.1
-->

## フロントエンドの改修

- 基本的にcodexにはrust側を頑張ってもらい、デザインなどもかかわってくるので自分はフロント側を改修する。
- とりあえずApp.tsx集権を解決するためストアを作成。

## ストア作成

- 思ったより保持という意味でのSignalは少なく、結局activeSessionとprofilesの二つだけか。あとbusyActionも？
- それでも少しは見通しが良くなったと思う。