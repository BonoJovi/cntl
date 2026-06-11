# cntl コマンドチュートリアル

> **対象**: cntl を初めて触る人。Git を多少知っていれば読みやすいですが、知らなくても読めます。
> **想定バージョン**: v0.1.0 (MVP)

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

## 7. 履歴を見る — `cntl log`

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

## 8. 全コマンド早見表

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
| `cntl commit -m "..."` | 全変更をまとめて 1 コミット | `[shorthash] message` |
| `cntl log` | HEAD から親をたどって履歴表示 | コミットブロック × N |

---

## 9. v0.1.0 でできないこと

以下は意図的に未実装です。次のバージョンで入ります ([ロードマップ](../README.md#ロードマップ) 参照)。

- ブランチ、`checkout`、`restore` — 単一履歴のみ
- `.cntlignore` — `.cntl/` 以外を除外するルールは未実装。一時ファイルもコミット対象になります
- リモート操作 (`push` / `pull` / `fetch`) — ローカル完結
- 2 層履歴のグルーピング — cntl の中核機能。v0.4.0 予定

---

## 次に読むもの

- [README](../README.md) — 全体像とロードマップ
- [CONCEPT](CONCEPT.md) — なぜこの設計なのか、設計思想の詳細
