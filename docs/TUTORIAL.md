# cntl コマンドチュートリアル

> **対象**: cntl を初めて触る人。Git を多少知っていれば読みやすいですが、知らなくても読めます。
> **想定バージョン**: v0.2.0 (開発中)

このドキュメントは、空のディレクトリから始めて、コミット履歴を作るところまでを順番にたどります。
全コマンドの出力例つきなので、自分の手元と照合しながら進めてください。

---

## 0. 準備 — `cntl` を使える状態にする

リポジトリをクローンしてビルドします。

```bash
git clone https://github.com/BonoJovi/cntl.git
cd cntl
cargo build --release
```

成果物は `target/release/cntl` に出ます。PATH を通すか、絶対パスで呼んでください。
以降の例では `cntl` コマンドが PATH 上にある前提で書きます。

```bash
cntl --version
# → cntl 0.1.0
```

---

## 1. リポジトリを作る — `cntl init`

任意のディレクトリで初期化します。

```bash
mkdir my-project
cd my-project
cntl init
```

期待される出力:

```
Initialized empty cntl repository in /absolute/path/to/my-project/.cntl
```

これで `.cntl/repo.db` という SQLite ファイルが 1 つだけ作られます。
cntl のすべての履歴・オブジェクト・参照はこの 1 ファイルに格納されます (Git の `.git/objects/` のような大量のファイルは生まれません)。

> **詰まりポイント**: 既に `.cntl` ディレクトリがあると `.cntl already exists in the current directory` で止まります。
> 既存のリポジトリを作り直したい場合は、自分で `.cntl/` を削除してから `cntl init` し直してください (履歴は消えます)。

---

## 2. 著者情報を設定する — `cntl config`

コミットには著者名とメールが必須です。設定していないと `cntl commit` がエラーで止まります。

### 2.1 グローバル設定 (推奨)

PC 全体で使う設定。普段はこちらで OK。

```bash
cntl config user.name "Your Name"
cntl config user.email "you@example.com"
```

出力は何も出ません (静かに成功)。値は `~/.config/cntl/global.db` に保存されます。

### 2.2 現在の値を確認する

値を省略すると読み取りモードになります。

```bash
cntl config user.name
# → Your Name
```

### 2.3 アクティブな設定を一覧する

今まさにコミットで使われる **実効値** をスコープつきで一覧表示します。

```bash
cntl config --all
```

```
user.email(local) "work@example.com"
user.name(global) "Your Name"
```

`(local)` / `(global)` の表示で、各値がどちらのスコープから来ているかが一目で分かります。
両方のスコープに同じキーがある場合は、影に隠れている値も `--verbose` で確認できます。

```bash
cntl config --all --verbose
```

```
user.email(local) "work@example.com"
  shadowed: global = "you@example.com"
user.name(global) "Your Name"
```

### 2.4 ローカル設定 (このリポジトリだけ)

特定のリポジトリだけ別の名前を使いたいとき。

```bash
cntl config --local user.email "work@example.com"
```

`--local` を付けると `.cntl/repo.db` 内の `settings` テーブルに書きます。
**読み取り順序は ローカル → グローバル** なので、ローカルがあれば優先されます。

> **詰まりポイント**:
> - `--local` 指定時にリポジトリ外 (`.cntl/` がない場所) で実行すると `not in a cntl repository` で止まります。
> - 未設定のキーを読もうとすると `config key not set: <key>` で止まります。

---

## 3. 状態を見る — `cntl status`

何が変わっているか確認します。`git status` と同じ感覚で使えます。

### 3.1 まだ何もない状態

```bash
cntl status
```

```
No commits yet.

nothing to commit (working tree empty)
```

### 3.2 ファイルを 1 つ作った状態

```bash
echo "hello cntl" > README.md
cntl status
```

```
No commits yet.

Untracked files:
	new file:   README.md
```

cntl には **`git add` に相当する明示的なステージング操作はありません**。
作業ツリーをスキャンして、HEAD と比較した差分をそのまま出します。

---

## 4. コミットする — `cntl commit -m`

```bash
cntl commit -m "first commit"
```

期待される出力:

```
[a1b2c3d4] first commit
```

`[...]` の中はコミットハッシュの先頭 4 バイト (8 文字) です。

### 内部で何が起きているか

1. 作業ツリー全体を走査
2. 各ファイルを **blob** として保存
3. ディレクトリ構造を **tree** として保存
4. それらをまとめた **commit** オブジェクトを作成
5. `HEAD` を新しい commit に進める

`.cntl/` ディレクトリ自体は自動的に無視されます (他の無視ルールは v0.1.0 では未実装)。

> **詰まりポイント**:
> - `user.name` や `user.email` が未設定だと、`user.name not configured (run \`cntl config user.name "..."\`)` で止まります。手順 2 に戻ってください。
> - `-m` は必須です。エディタを開く動作 (`git commit` のような) はまだありません。

---

## 5. 差分のサイクルを回す

実際の作業では、編集 → status → commit を繰り返します。

```bash
echo "## Usage" >> README.md
echo "TODO" > NOTES.md
cntl status
```

```
Changes since last commit:
	modified:   README.md
	new file:   NOTES.md
```

そのままコミット:

```bash
cntl commit -m "add NOTES, expand README"
```

ファイルを消したらどうなるか:

```bash
rm NOTES.md
cntl status
```

```
Changes since last commit:
	deleted:    NOTES.md
```

`modified` / `new file` / `deleted` の 3 種類が、cntl が現在認識する変更の全種類です。

---

## 6. 内容差分を見る — `cntl diff`

`cntl status` で変更されたファイルの一覧は分かりますが、**中身がどう変わったか** までは出ません。`cntl diff` を使うと、HEAD と作業ツリーの内容差分を unified diff 形式で表示します。

### 6.1 編集してから diff

```bash
echo "first line" >> README.md
echo "## TODO" > NOTES.md
cntl diff
```

```
diff --cntl a/NOTES.md b/NOTES.md
--- /dev/null
+++ b/NOTES.md
@@ -0,0 +1 @@
+## TODO
diff --cntl a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1,2 @@
 hello cntl
+first line
```

- **`diff --cntl a/path b/path`** — ファイルごとのヘッダ
- **`--- a/path`** / **`+++ b/path`** — 旧側 / 新側のラベル
- **`@@ -行,数 +行,数 @@`** — ハンクヘッダ (Git と同じフォーマット)
- 行頭の **`-`** / **`+`** が削除 / 追加、空白で始まる行は文脈行

### 6.2 新規ファイル・削除ファイル

新規ファイルは旧側が `/dev/null` に、削除ファイルは新側が `/dev/null` になります。

```bash
rm NOTES.md
cntl diff
```

```
diff --cntl a/NOTES.md b/NOTES.md
--- a/NOTES.md
+++ /dev/null
@@ -1 +0,0 @@
-## TODO
```

### 6.3 バイナリファイル

NUL バイトを含むファイル (画像・実行ファイル等) は中身を出さず、1 行で終わります。

```
diff --cntl a/image.png b/image.png
Binary files a/image.png and b/image.png differ
```

判定は **先頭 8KB に NUL バイトがあるか** というシンプルなヒューリスティック (Git と同じ) です。

### 6.4 変更がないとき

何も出力されません (`git diff` と同じ振る舞いです)。

> **詰まりポイント**:
> - パス引数で絞り込む形式 (`cntl diff README.md`) は v0.1.x では未実装で、常に全変更が出ます。
> - 任意の 2 コミットを比較する `cntl diff <commit> <commit>` は v0.2.0 以降の予定です。
> - 着色 (`--color`) や `--stat` 等のオプションはまだありません。

---

## 7. 取り消す — `cntl restore`

作業中に「やっぱり今の変更を捨てたい」「うっかり消したファイルを戻したい」というとき、`cntl restore` を使います。指定したパスを **HEAD の状態に戻す** (＝最後のコミット時点の内容で上書き) コマンドです。

### 7.1 編集を取り消す

```bash
echo "壊れた内容" > README.md
cntl diff
```

```
diff --cntl a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1 @@
-hello cntl
+壊れた内容
```

```bash
cntl restore README.md
cntl diff
# → (何も出ない、HEAD と一致)
```

成功時は何も出力しません (`git restore` と同じ静かな成功)。

### 7.2 消したファイルを戻す

ファイルそのものが消えていても、HEAD に存在していれば復元できます。

```bash
rm docs/TUTORIAL.md
cntl status
```

```
Changes since last commit:
	deleted:    docs/TUTORIAL.md
```

```bash
cntl restore docs/TUTORIAL.md
cntl status
# → nothing to commit, working tree clean
```

親ディレクトリ (`docs/` 自体) が消えていても自動的に作り直されます。

### 7.3 複数パスを一度に

引数は何個でも渡せます。

```bash
cntl restore README.md docs/TUTORIAL.md
```

### 7.4 「半分だけ復元」は起きない

引数を **すべて先に検証** してから書き込みに入る 2 フェーズ方式です。

```bash
cntl restore README.md not-existing.txt
# → Error: not-existing.txt: not in HEAD
# README.md には触らない (作業ツリーは変化なし)
```

1 つでも HEAD に無いパスが混ざっていたら、その時点で全体を中止します。

> **詰まりポイント**:
> - **ディレクトリ指定はまだサポートしていません** (`cntl restore docs` → `is a directory (not yet supported)`)。ディレクトリ以下を一括 restore するには、ファイル名を 1 つずつ列挙してください。
> - **任意コミットからの restore** (`git restore --source=<rev>`) は v0.2.0 以降の予定です。今は常に HEAD から。
> - **コミット 0 個の状態では使えません** (`no commits yet`)。

---

## 8. 履歴を見る — `cntl log`

```bash
cntl log
```

```
commit a1b2c3d4e5f6...（64文字フル）
Author: Your Name <you@example.com>
Date:   2026-06-09 15:42:11 +0900

    add NOTES, expand README

commit 0011223344556677...
Author: Your Name <you@example.com>
Date:   2026-06-09 15:30:00 +0900

    first commit
```

新しい順 (HEAD → 親 → 親の親 …) に並びます。

- ハッシュは full (64 文字、blake3) で表示
- 日時は **保存は UTC、表示はローカルタイムゾーン**

> **詰まりポイント**: コミットが 1 つもない状態で `cntl log` を打つと `no commits yet` で止まります。

---

## 9. ブランチを管理する — `cntl branch`

cntl は v0.2.0 からブランチを扱えます。ブランチは「履歴の枝分かれ先 (tip) を指す名前付きポインタ」で、Git とほぼ同じ概念です。ただし cntl では **HEAD は常にブランチを指す** という不変条件があり、「うっかり detached HEAD」が起きない設計です ([DR-002](adr/DR-002-head-and-branches.md) 参照)。

### 9.1 ブランチの一覧 — `cntl branch`

引数なしで実行すると、現在の全ブランチを表示します。現在いるブランチには行頭に `*` が付きます。

```bash
cntl branch
```

```
* main
```

最初のコミットを打つまではブランチ ref がまだ存在しないので、一覧はこんな表示になります:

```
No branches yet (HEAD will become 'main' at first commit).
```

これは「`cntl init` 直後の `HEAD` は `refs/heads/main` を指しているが、`main` ブランチ自体は最初の commit で初めて生まれる」という Git と同じ挙動です。

### 9.2 ブランチを作る — `cntl branch <name>`

現在の HEAD commit を基点に、新しいブランチを作成します。**作成だけで、切り替えはしません** (切り替えは将来の `cntl checkout` で。v0.2.0 では未実装)。

```bash
cntl branch feature-login
cntl branch
```

```
  feature-login
* main
```

成功時は何も出力しません (`git branch` と同じ静かな成功)。一覧は名前順 (ASCII 順) で並びます。

### 9.3 ブランチを消す — `cntl branch -d <name>`

不要になったブランチを削除します。短縮形 `-d` と長形 `--delete` のどちらでも同じ意味です。

```bash
cntl branch -d feature-login
cntl branch
```

```
* main
```

削除されるのは **ブランチ ref のみ** で、過去の commit オブジェクトは `objects` テーブルにそのまま残ります。別ブランチから到達できる commit なら、引き続き `cntl log` で見られます。

### 9.4 ブランチ名のルール

最小限ですが、以下は弾かれます:

| 不正な名前 | 理由 |
|---|---|
| 空文字列 | 名前として成立しない |
| `.` / `..` | パス的に紛らわしい |
| `/` を含む | 階層的なブランチ名は将来予約 |
| 空白・タブ・改行を含む | shell/UI で混乱の元 |
| 制御文字を含む | 同上 |

> **詰まりポイント**:
> - **先頭が `-` の名前**: `cntl branch -x` のように打つと clap が `-x` をオプションとして解釈しようとして、cntl の検証より手前で蹴られます。どうしても先頭 `-` を使いたい場合は `cntl branch -- -x` と書きますが、おすすめしません。
> - **初コミット前の `cntl branch <name>`**: `no commits yet — cannot create a branch` で止まります。基にする commit が無いためです。手順 4 に戻ってまず 1 つコミットしてください。
> - **現在いるブランチを `-d` で消す**: `cannot delete branch '<name>': it is the current branch` で止まります。これは安全側の制約で、HEAD が指すブランチ ref が消える状態を構造的に防いでいます ([DR-002](adr/DR-002-head-and-branches.md) D3 不変条件)。先に別ブランチを作ってそちらに切り替えてから消す、というのが正規ルートになります (`checkout` の実装後に手順化予定)。

### 9.5 ブランチを作っただけでは履歴は分岐しない

ここは初学者が混乱しやすいポイントです。`cntl branch feature` を打った直後の状態は:

```
        main (current, HEAD)
         ↓
  commit C ← commit B ← commit A
         ↑
       feature (新規作成、同じ commit を指す)
```

`main` と `feature` は **同じ commit C を指している 2 つの名前付きポインタ** にすぎません。実際の履歴分岐は、片方のブランチに切り替えてから新しい commit を打ったときに発生します。それは `cntl checkout` の実装後に詳述します。

---

## 10. 全コマンド早見表

| コマンド | 何をするか | 出力 |
|---|---|---|
| `cntl init` | カレントに `.cntl/repo.db` を作る | `Initialized empty cntl repository in ...` |
| `cntl config <key> <value>` | グローバルに設定を書き込む | (なし) |
| `cntl config --local <key> <value>` | このリポジトリだけの設定を書き込む | (なし) |
| `cntl config <key>` | 設定値を読み取る (ローカル→グローバル順) | 値 1 行 |
| `cntl config --all` | アクティブな設定をスコープつきで一覧 | `key(scope) "value"` × N |
| `cntl config --all --verbose` | 上記 + 上書きされた値も表示 | 同上 + `shadowed: ...` 行 |
| `cntl status` | 作業ツリーと HEAD の差分を表示 | `modified` / `new file` / `deleted` 各行 |
| `cntl diff` | 作業ツリーと HEAD の内容差分を unified diff で表示 | `diff --cntl ...` ブロック × N |
| `cntl restore <path>...` | 指定パスを HEAD の内容で復元 (作業ツリー変更の取り消し) | (なし) |
| `cntl commit -m "..."` | 全変更をまとめて 1 コミット | `[shorthash] message` |
| `cntl log` | HEAD から親をたどって履歴表示 | コミットブロック × N |
| `cntl branch` | ブランチ一覧 (現在のブランチに `*`) | `* main` などの行 × N |
| `cntl branch <name>` | HEAD commit を基点に新規ブランチを作成 | (なし) |
| `cntl branch -d <name>` | ブランチを削除 (現在のブランチは不可) | (なし) |

---

## 11. v0.2.0 でできないこと

以下は意図的に未実装です。次のバージョンで入ります ([ロードマップ](../README.md#ロードマップ) 参照)。

- `cntl checkout <branch>` — ブランチ切り替え。v0.2.0 で実装予定 (`branch` は作るところまで)
- `cntl restore --source=<commit> <path>` — 任意のコミットからの復元。現在は HEAD からのみ
- `cntl restore` のディレクトリ指定 — 1 ファイルずつのみサポート
- `.cntlignore` — `.cntl/` 以外を除外するルールは未実装。一時ファイルもコミット対象になります
- conflict 解消モード、inspect モード (任意 commit の閲覧) — v0.3.0 予定
- リモート操作 (`push` / `pull` / `fetch`) — ローカル完結
- 2 層履歴のグルーピング — cntl の中核機能。v0.4.0 予定

---

## 次に読むもの

- [README](../README.md) — 全体像とロードマップ
- [CONCEPT](CONCEPT.md) — なぜこの設計なのか、設計思想の詳細
- [設計判断記録 (DR)](adr/index.md) — 個別実装判断の Why (ストレージ設計、HEAD とブランチのモデル等)
