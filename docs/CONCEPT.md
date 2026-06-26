# cntl (コントル) — 一風変わったVCS

## プロジェクト名

**cntl** (コントル)

GitHub上のリポジトリ初期化時に置かれたハッシュ実験プロジェクト名を継承。
content-addressable storage のハッシュ実験 (`cntl.test/` にローカル保持) が前身。

## 概要

Gitとは別設計の独自バージョン管理システム (VCS)。Rust製、CLI + TUIベース。

「Gitに似ているが別物」を明確にし、Gitの摩擦点を解消することと、独自コンセプト
（特に2層履歴モデル）を試すことを両立させる、設計実験的なプロジェクト。

## モチベーション

- GitのUX/CLIが複雑すぎるのをシンプル化したい
- Gitの内部データモデルを見直したい
- 独自のビジョン/コンセプトを試したい
- 学習・趣味・チャレンジとして

## ターゲット

未確定。実装を進めながら決定する。
（現時点ではソロ開発者のローカルシンク〜小規模チーム共同作業の範囲を想定）

---

## 設計コンセプト

### 1. 2層履歴モデル (cntlの中核アイデンティティ)

履歴を二層構造で管理する:

- **下層 — 作業コミット**: 細かく頻繁に打つ。実装メモ的、雑でOK
- **上層 — グループ**: 論理単位（機能/PR/リリース等）

**重要**: グルーピングは **後付け** で可能。Gitのsquash/rebaseと違い、元コミットは保持される（履歴破壊なし）。

#### グルーピング操作
- 範囲指定: `cntl group <commit1>..<commit5> -m "feat: add login"`
- 対話的: `cntl group --interactive` で TUI 起動して選択
- 両方サポート

#### グループ構造
- **入れ子可能** ( `Group { children: Vec<Either<Commit, Group>> }` )
- 階層例: リリース ⊃ 機能 ⊃ サブ機能

#### リモートとの同期
- コミットもグループも両方push（リモートでも2層構造を完全保持）
- 受信側でも `cntl log` でグループ単位の整理された履歴を見られる

#### 解決される痛み
- 「細かくコミットしたいけど履歴が汚くなる」というジレンマの解消
- 作業中の安心感（rollback先が細かく取れる）と読みやすい履歴の両立

---

### 2. タグはブランチコンテキスト付き

**Gitの問題:**
タグがブランチ非依存（branch-agnostic）なため、「v1.0がmain由来かhotfix由来か」がタグだけでは追えない。

**cntlの設計:**

- **ブランチスコープ付きタグ**: タグ名が `branch/tag_name` のスコープ付き
  - `main/v1.0` と `dev/v1.0` が共存可能
  - グローバル名前空間ではない
- **タグ作成イベントの履歴を記録**: 「いつ、どのブランチで、誰がタグを打ったか」を別途追跡
  - タグ自体は不変だが、タグ作成イベントは履歴として残る

---

### 3. conflict はメタデータ分離 + TUIモード解消

**Gitの問題:**
- conflict marker (`<<<<<<<` / `=======` / `>>>>>>>`) をファイルに直接埋め込むため、ファイルが「壊れた」状態になる
- 解消場所はgrep頼り、conflict解消UXは古い

**cntlの設計:**

#### conflict状態の保存場所
- **メタデータ分離**: ファイル自体にmarkerを埋め込まない
- conflictチャンクは `.cntl/conflicts/` 等に構造化して保持
- **作業ファイルは常にコンパイル可能/lint通る状態を維持**
  - 副作用: conflict中でも `cargo build` / テスト / lint が動く
  - IDEの赤波線地獄を回避

#### conflict解消モード (TUI)
- `cntl mode resolve` で TUI 起動
- `cntl mode browse` で通常モードに戻る (Tab/Esc でも可)
- 想定キーバインド:
  - `n` / `p`: 次/前のconflictへジャンプ
  - `o` / `t` / `b`: ours / theirs / both を選択
  - `e`: 手動編集
  - `c`: 周辺コードを±N行展開して表示
  - `s`: 現在のconflictを「解消済み」マーク

#### スコープ
- **初期版はTUIオンリー** (実装負荷を抑える)
- エディタ連携は将来の拡張として保留

---

### 4. ステージングは自動検出 + 手動オーバーライド

**Gitの問題:**
- `git add` という中間概念の摩擦
- `git add .` / `git add -A` で意図しないファイルが混入する事故

**cntlの設計:**

- **デフォルト**: 変更ファイル＆新規ファイルは自動でステージング対象
  - `.cntlignore` で除外
- **オーバーライド**: 個別 `include` / `exclude` で手動制御可能
- **部分コミット (hunk-level) は初期実装では非対応**
  - 理由: 「より細かくコミットする」運用方針なので、ファイル単位で十分という判断
  - hunk-level が必要になっても、2層履歴のグルーピングで代替可能

---

---

## MVP (v0.1.0) スコープ — Walking Skeleton

最小限の「ローカルで単一履歴を記録できる」レベル。

### 含むコマンド
- `cntl init` — 新規リポジトリ作成 (`.cntl/cntl.db` を生成)
- `cntl config user.name "..."` / `user.email "..."` — 著者情報設定
- `cntl status` — 作業ツリーの変更を表示 (HEAD と比較)
- `cntl commit -m "..."` — 全変更を自動ステージングして 1 コミット
- `cntl log` — コミット履歴を表示

### MVP に含まないもの (v0.2 以降)
- ブランチ / merge / rebase
- リモート操作 (push/pull/fetch)
- conflict (merge を持たないので発生しない)
- 2層履歴のグルーピング
- タグ
- ステージングの手動オーバーライド (まずは全自動のみ)
- diff / checkout / restore

---

## 技術スタック (確定)

| 領域 | 採用 | 理由 |
|---|---|---|
| 言語 | Rust (edition 2024) | パフォーマンス、型安全性 |
| ハッシュ | **blake3** | 高速・暗号学的・モダン (SHA-256 より高速) |
| シリアライズ | **bincode** + serde | Rust 事実上標準、コンパクト |
| ストレージ | **SQLite** (rusqlite) | 2 層 DB (グローバル設定 + per-repo)、ACID、Fossil/Sapling 路線 |
| CLI | **clap** (derive macro) | エコシステム標準 |
| 設定ディレクトリ解決 | directories | XDG/macOS/Windows 標準パス取得 |
| 作業ツリースキャン | walkdir | 軽量・標準的 |
| 日時 | chrono | UTC 保存、表示時にローカル TZ へ変換 |
| TUI (将来) | ratatui 等 | conflict 解消モードで採用予定 |

---

## オブジェクトモデル (MVP)

Git 同様の 3 種類のオブジェクトを採用 (v0.2+ で patch-based モデルへの段階移行を検討):

- **blob**: ファイル内容のスナップショット
- **tree**: ディレクトリスナップショット (エントリ名 → blob/tree ハッシュのマップ)
- **commit**: parent commit hash + tree hash + author + timestamp (UTC) + message

すべて bincode でシリアライズして SQLite に格納。

### 作業ツリー追跡
- **index なし** (MVP)。`cntl status` 実行時に作業ツリーを毎回スキャンして HEAD tree と比較
- 大規模リポでパフォーマンス問題が出た時に index 化を検討 (v0.2+)

---

## ストレージ構造 (MVP) — 2 層 DB 構造

**設計意図**: グローバル設定とリポジトリデータを分離して、

- 大規模リポのパフォーマンス影響を他リポに波及させない
- DB ファイル破損の影響範囲を1リポに限定
- バックアップ/転送単位がリポジトリ単位で完結
- グローバル設定を全リポで共有 (毎回 `cntl config` し直さなくて済む)

### グローバル DB

- **場所**: XDG 準拠 (`directories` クレートでクロスプラットフォーム対応)
  - Linux: `~/.config/cntl/global.db`
  - macOS: `~/Library/Application Support/cntl/global.db`
  - Windows: `%APPDATA%/cntl/global.db`
  - 環境変数 `CNTL_HOME` で上書き可能
- **役割**: 全リポジトリで共有する user-level 設定
- **スキーマ**:

```sql
CREATE TABLE settings (
    key    TEXT PRIMARY KEY,   -- 'user.name', 'user.email' 等
    value  TEXT NOT NULL
);
```

### リポジトリ DB

- **場所**: `<repo>/.cntl/repo.db` (1 ファイル = 1 リポジトリ)
- **役割**: そのリポのオブジェクト・refs・リポ固有設定
- **スキーマ**:

```sql
CREATE TABLE objects (
    hash      BLOB PRIMARY KEY,  -- blake3 hash (32 bytes)
    obj_type  TEXT NOT NULL,     -- 'blob' | 'tree' | 'commit'
    data      BLOB NOT NULL      -- bincode-serialized payload
);

CREATE TABLE refs (
    name      TEXT PRIMARY KEY,  -- v0.1.0 は 'HEAD' のみ
    target    BLOB NOT NULL      -- commit hash
);

CREATE TABLE settings (
    key       TEXT PRIMARY KEY,  -- リポ固有の設定オーバーライド
    value     TEXT NOT NULL
);
```

### `cntl config` の動作

- `cntl config user.name "..."` → デフォルトで**グローバル DB** に書く
- `cntl config --local user.name "..."` → **リポジトリ DB** に書く (グローバルを上書き)
- 読み取り順序: **local → global → 未設定エラー** (Git と同じ優先順位)

---

## ロードマップ

| Version | 内容 |
|---|---|
| v0.1.0 (MVP) | Walking skeleton: init / config / status / commit / log |
| **v0.2.0 ← 開発中** | restore ✅ / diff ✅ / branch ✅ / checkout ✅ — HEAD はシンボリック参照化済み ([DR-002](adr/DR-002-head-and-branches.md)) |
| v0.3.0 | conflict メタデータ分離 + TUI 解消モード、inspect モード (detached HEAD 相当の明示モード) |
| v0.4.0 | 2 層履歴 (グルーピング)、branch-scoped タグ |
| v0.5.0 | リモート操作 (push/pull/fetch) |
| 将来 | patch-based モデルへの移行 (Pijul/Darcs 風) |

---

## 差別化ポイント

| 項目 | Git | cntl |
|---|---|---|
| 履歴 | 1 層、squash/rebase は破壊的 | 2 層、グルーピングは非破壊 |
| ステージング | 必須の中間概念 | 自動検出、必要時のみ手動 |
| conflict markers | ファイルに埋め込み | メタデータ分離、ファイル無傷 |
| タグ | branch-agnostic | branch-scoped + 履歴 |
| 解消 UX | 外部ツール任せ | 第一級の TUI モード |
| ストレージ | ファイルベース (objects/) | 単一 SQLite DB |

## 設計原則

- **摩擦を減らす**: 日常運用で考えなくて済むことは自動化
- **履歴を壊さない**: 後付けで整理できるが、元の事実は残る
- **ファイルを壊さない**: VCS の内部状態が作業ファイルを汚さない
- **CLI/TUI 両対応**: スクリプト化容易、対話的操作も快適

---

## 未確定事項 (今後詰める)

- コミットメッセージのフォーマット規約 (conventional commits か独自か)
- ターゲットスケール (規模上限の想定)
- 認証/転送プロトコル (v0.5 リモート設計時)
- inspect モードの具体 (名前 / 入退出コマンド / 内部で許可される操作) — 通常モードでは detached 不可とする方針は [DR-002](adr/DR-002-head-and-branches.md) で確定済み、具体は v0.3 までに別 DR で決定
- `log` / `diff` 表示の粒度と既定 (実装時に決定)
