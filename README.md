# cntl

> **A Rust-based VCS with a two-tier history model — by [BonoJovi](https://github.com/BonoJovi).**

**cntl** (コントル) は、Git とは別設計の独自バージョン管理システム (VCS) です。Rust 製、CLI + (将来) TUI ベース。
「Git に似ているが別物」を明確にし、Git の摩擦点を解消することと、独自コンセプト (特に **2 層履歴モデル**) を試すことを両立させる、設計実験的なプロジェクト。

開発者: **BonoJovi** ([@BonoJovi](https://github.com/BonoJovi)) — Rust 製プロダクトを連続リリース中 (KakeiBonByRust / Promps-Ent / ai2ia / JaIM / cntl)。

---

## 状態

**v0.2.0** — v0.1.0 (MVP) のローカル単一履歴に加え、ブランチ管理 (作成 / 切り替え) が動作します。

| コマンド | 状態 | 役割 |
|---|---|---|
| `cntl init` | ✅ | 新規リポジトリ作成 (`.cntl/repo.db` を生成) |
| `cntl config <key> [value]` | ✅ | 著者情報設定 (グローバル / ローカル二層) |
| `cntl status` | ✅ | 作業ツリーと HEAD の差分を表示 |
| `cntl diff` | ✅ | 作業ツリーと HEAD の内容差分を unified diff 形式で表示 |
| `cntl restore <path>...` | ✅ | 指定パスを HEAD の内容で復元 (変更の取り消し) |
| `cntl commit -m "..."` | ✅ | 全変更を自動ステージングして 1 コミット |
| `cntl log` | ✅ | コミット履歴を表示 |
| `cntl branch [name] [-d]` | ✅ | ブランチの一覧 / 作成 / 削除 |
| `cntl checkout <branch>` | ✅ | ブランチ切り替え (作業ツリーを対象ブランチに一致させる) |

---

## なぜ cntl か — Git との差別化ポイント

| 項目 | Git | cntl |
|---|---|---|
| 履歴 | 1 層、squash/rebase は破壊的 | **2 層、グルーピングは非破壊** |
| ステージング | `git add` という必須の中間概念 | **自動検出、必要時のみ手動** |
| conflict マーカー | ファイルに `<<<<<<<` 埋め込み | **メタデータ分離、ファイル無傷** |
| タグ | branch-agnostic | **branch-scoped + 履歴を記録** |
| conflict 解消 | 外部ツール任せ | **第一級の TUI モード** (予定) |
| ストレージ | ファイルベース (`.git/objects/`) | **単一 SQLite DB** (`.cntl/repo.db`) |

設計の詳細・思想は [docs/CONCEPT.md](docs/CONCEPT.md) を参照。

---

## クイックスタート

```bash
# ビルド
git clone https://github.com/BonoJovi/cntl.git
cd cntl
cargo build --release
# 生成物: target/release/cntl

# 使う
cd path/to/your/project
cntl init
cntl config user.name "Your Name"
cntl config user.email "you@example.com"

echo "hello" > foo.txt
cntl status                      # → Untracked: foo.txt
cntl commit -m "first commit"    # → [<hash>] first commit
cntl log                         # → コミット履歴
```

各コマンドの詳しい使い方は [docs/TUTORIAL.md](docs/TUTORIAL.md) を参照。

---

## 技術スタック

| 領域 | 採用 |
|---|---|
| 言語 | **Rust** (edition 2024) |
| ハッシュ | **blake3** |
| シリアライズ | **postcard** + serde |
| ストレージ | **SQLite** (rusqlite, bundled) |
| CLI | **clap** (derive macro) |
| 日時 | chrono (UTC 保存・ローカル TZ 表示) |
| エラー | anyhow |
| TUI (将来) | ratatui 等 |

---

## ロードマップ

| Version | 内容 |
|---|---|
| v0.1.0 (MVP) | Walking skeleton: init / config / status / diff / restore / commit / log |
| v0.2.0 ✅ | ブランチ操作: branch / checkout |
| **v0.3.0 ← 次** | merge (conflict 無し版): fast-forward + 非衝突統合 |
| v0.4.0 | conflict 解消: メタデータ分離 + TUI 解消モード |
| v0.5.0 | inspect モード (任意 commit の明示閲覧 — DR-002 参照) + `restore --source=<commit>` |
| v0.6.0 | 2 層履歴 (グルーピング)、branch-scoped タグ |
| v0.7.0 | リモート操作 (push/pull/fetch) |
| 将来 | patch-based モデルへの移行 (Pijul/Darcs 風) |

---

## ライセンス

MIT License. 詳細は [LICENSE](LICENSE) を参照。

---

## 関連

- 使い方チュートリアル: [docs/TUTORIAL.md](docs/TUTORIAL.md)
- 設計ドキュメント: [docs/CONCEPT.md](docs/CONCEPT.md)
- 設計判断記録 (DR): [docs/adr/index.md](docs/adr/index.md)
- 開発者の他プロダクト (Rust 製):
  - **KakeiBonByRust** — 家計簿アプリ
  - **Promps-Ent** — ブロック志向のプロンプトジェネレータ
  - **ai2ia** — プロンプトジェネレータ + AI 比較ツール
  - **JaIM** — Linux 専用の日本語 Input Method
