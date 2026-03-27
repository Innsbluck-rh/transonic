
ちょっと手戻りというか再確認なんですが、この時話した
> foobar2000/Feishin 的な folder-like library browser
について、具体的な関連APIや実装方法を(Open) Subsonic APIに基づいて教えてください。


<!-- 以下、初期構想を話していたChatGPTからの回答 -->


思考時間: 14s

できます。
この用途では、実装の中心はかなりはっきりしています。

結論

foobar2000/Feishin 的な “folder-like library browser” を OpenSubsonic / Subsonic API 上で作る場合、主軸になるのは getMusicFolders・getIndexes・getMusicDirectory です。
OpenSubsonic 公式 docs でも、ID3ベースの閲覧 (getArtists / getArtist / getAlbum) と、file structure ベースの閲覧 (getIndexes / getMusicDirectory) は別系統として整理されています。

ただし、前回確認した通り、Navidrome では browse-by-folder は本物の実フォルダではなく simulated directory tree です。
公式 docs でも getIndexes / getMusicDirectory は /Artist/Album/01 - Song.mp3 形式の疑似ツリーを返すと書かれています。

なので実装方針としては、

API上は file-structure 系 endpoint を使う
UI上は folder-like に見せる
意味論としては「実filesystemブラウザ」ではなく「階層ブラウザ」だと割り切る

この3点で進めるのが自然です。

使うAPI
1. getMusicFolders

役割は、トップレベルの music folder 一覧の取得です。
複数ライブラリ対応サーバでは、ここがライブラリ選択の起点になります。OpenSubsonic / Subsonic の browsing 系 endpoint に含まれています。

実装上の意味は次です。

初回接続時に取得
ユーザーに複数ライブラリを選ばせる場合の候補
あるいは内部で既定ライブラリを決めるための起点
2. getIndexes

役割は、アーティスト単位の最上位階層を取ることです。
Subsonic / OpenSubsonic docs ではこれが file structure browsing 側の入口として扱われています。

実装上は、ここで取れるものを

左ペインの tree
A/B/C… のインデックス
アーティスト一覧

として使います。

ただし注意点として、Navidrome の場合この階層は real folder ではなく simulated です。
つまり「アーティストフォルダのように見える最上位ノード」であって、OS のディレクトリエントリそのものと同一視しない方がよいです。

3. getMusicDirectory

役割は、ある directory ID の中身を列挙することです。
OpenSubsonic docs でも「artist の album 一覧」や「album の song 一覧」に典型的に使うと説明されています。返る ID は getIndexes または getMusicDirectory 由来です。

この endpoint が実質的にブラウザ本体です。

artist ノードを開く
→ album 相当の子ノードを表示
album ノードを開く
→ song 一覧を表示
必要ならさらに下層があれば継続
基本的な画面モデル

folder-like browser を作るなら、API から見ると次の3層でほぼ足ります。

層1: ライブラリ
取得元: getMusicFolders
UI: サーバ内ライブラリの切り替え
層2: アーティスト一覧
取得元: getIndexes
UI: tree 左ペイン、または artist list
層3: 中身の展開
取得元: getMusicDirectory
UI:
アルバム一覧
曲一覧
mixed list
パンくず

この構造は、“file structure API を使うが、UIとしては artist → album → tracks に整形する” という作りです。
Feishin 的にも foobar2000 的にも、この考え方でかなり近い体験になります。

実装フロー
初期化
ping か同等の疎通確認
server 情報取得
getMusicFolders
getIndexes

OpenSubsonic は既存 Subsonic API と後方互換を保ちながら、server identification や extension indication を目標にしています。
そのため、可能なら OpenSubsonic 拡張対応サーバでは識別情報や拡張情報を見て capability を切り替える設計がやりやすいです。

ツリー表示
getIndexes の結果をアーティストノードとして表示
ノードは最初は未展開
ノード展開
ノード選択時に getMusicDirectory(id)
結果の child を見て
ディレクトリ系なら次のノード
曲ならトラック一覧
曲表示
タイトル、時間、アルバム、年、カバー、番号などを整形表示
queue 追加や即再生に接続
実装で意識した方がいい点
1. 「folder」と「album」を内部で別概念にしすぎない

この用途では、見た目がフォルダでも実態は

アーティスト相当ノード
アルバム相当ノード
曲相当ノード

で十分です。

つまり内部モデルはむしろ

BrowserNode(kind = container | song)
ContainerSubtype = library | artistLike | albumLike | genericDir

くらいの曖昧さで持つ方が安全です。

理由は、Navidrome のように simulated tree を返すサーバでは、見た目のフォルダが必ずしも実フォルダではないからです。

2. ノードは「遅延展開」が自然

getMusicDirectory は階層を1段ずつ掘る用途なので、tree 全展開前提より lazy load 前提のほうが合います。

3. パンくずは client 側で持つ

Subsonic 系 API は「この ID の親を完全に返す」ことより、指定 ID の listing を返す 方向です。
そのため、UI のパンくずは

選択履歴
親ノード参照
ノードキャッシュ

で client 側に持つ方が実装しやすいです。

4. ソートは client 側で握る

folder-like browser の快適さは

track number
title
year
album
recently added

などの整列にかなり依存します。
API の生順序に寄せすぎず、表示ソートは client 側で調整できるようにしておく方がよいです。

OpenSubsonic と Subsonic の扱い
基本方針

まずは Subsonic 互換として成立させるのがよいです。
OpenSubsonic はそれに対して後方互換を維持する方針で、追加機能は optional です。

つまり最初は:

getMusicFolders
getIndexes
getMusicDirectory
stream
getCoverArt

あたりで十分土台になります。
その後で OpenSubsonic の

server identification
extensions
新認証や POST 対応

を足す流れがよいです。OpenSubsonic では application/x-www-form-urlencoded POST の公式サポートも追加されています。

そのまま設計に落とすなら

実装単位としては、次の責務分割がきれいです。

A. Transport
認証
バージョン
f=json
エラーハンドリング
Subsonic / OpenSubsonic 差分吸収
B. Browser Repository
getMusicFolders
getIndexes
getMusicDirectory
レスポンス正規化
ノードキャッシュ
C. Browser ViewModel
tree state
expanded state
selected node
breadcrumb
sort / filter
D. Playback Bridge
stream
queue 追加
song メタデータ解決

この分け方だと、folder-like browser は Browser Repository + ViewModel でほぼ閉じるので扱いやすいです。

実装上の割り切り

この用途では、次の割り切りがかなり効きます。

“フォルダ”は UI ラベルであって、内部的には browse node
artist/album はタグ由来でもよい
サーバが返す tree をそのまま受けつつ、client 側で見やすく整形する
filesystem faithful であることは要件に入れない

これで Feishin/foobar2000 的な体験にかなり寄せられます。
一方で、Poweramp 的な「生フォルダビュー」を狙う場合は別要件になります。

最低限必要な endpoint 群

本当に絞ると、folder-like library browser の最小セットはこれです。

getMusicFolders
getIndexes
getMusicDirectory
getCoverArt
stream

これで

ライブラリ選択
tree 表示
album/track 展開
ジャケット表示
再生

まで到達できます。 browsing 系の中心が getMusicFolders / getIndexes / getMusicDirectory であることは OpenSubsonic と Subsonic の docs 両方で確認できます。

必要なら次に、
この browser 用の内部データモデルをどう切るか
まで、型設計レベルで整理します。