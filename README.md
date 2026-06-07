# cntl

> **A Rust-based VCS with a two-tier history model — by [BonoJovi](https://github.com/BonoJovi).**

**cntl** (コントル) は、Git とは別設計の独自バージョン管理システム (VCS) です。Rust 製、CLI + (将来) TUI ベース。
「Git に似ているが別物」を明確にし、Git の摩擦点を解消することと、独自コンセプト (特に **2 層履歴モデル**) を試すことを両立させる、設計実験的なプロジェクト。

開発者: **BonoJovi** ([@BonoJovi](https://github.com/BonoJovi)) — Rust 製プロダクトを連続リリース中 (KakeiBonByRust / Promps-Ent / ai2ia / JaIM / cntl)。

---

## 状態

**v0.1.0 (MVP — Walking Skeleton)** — ローカルで単一履歴を記録できる最小限のセットが動作します。

| コマンド | 状態 | 役割 |
|---|---|---|
| `cntl init` | ✅ | 新規リポジトリ作成 (`.cntl/repo.db` を生成) |
| `cntl config <key> [value]` | ✅ | 著者情報設定 (グローバル / ローカル二層) |
| `cntl status` | ✅ | 作業ツリーと HEAD の差分を表示 |
| `cntl commit -m "..."` | ✅ | 全変更を自動ステージングして 1 コミット |
| `cntl log` | ✅ | コミット履歴を表示 |

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

---

## 技術スタック

| 領域 | 採用 |
|---|---|
| 言語 | **Rust** (edition 2024) |
| ハッシュ | **blake3** |
| シリアライズ | **bincode** + serde |
| ストレージ | **SQLite** (rusqlite, bundled) |
| CLI | **clap** (derive macro) |
| 日時 | chrono (UTC 保存・ローカル TZ 表示) |
| エラー | anyhow |
| TUI (将来) | ratatui 等 |

---

## ロードマップ

| Version | 内容 |
|---|---|
| **v0.1.0 (MVP)** ← 現在 | Walking skeleton: init / config / status / commit / log |
| v0.2.0 | branch、checkout、restore、diff |
| v0.3.0 | conflict メタデータ分離 + TUI 解消モード |
| v0.4.0 | 2 層履歴 (グルーピング)、branch-scoped タグ |
| v0.5.0 | リモート操作 (push/pull/fetch) |
| 将来 | patch-based モデルへの移行 (Pijul/Darcs 風) |

---

## ライセンス

MIT License. 詳細は [LICENSE](LICENSE) を参照。

---

## 関連

- 設計ドキュメント: [docs/CONCEPT.md](docs/CONCEPT.md)
- 開発者の他プロダクト (Rust 製):
  - **KakeiBonByRust** — 家計簿アプリ
  - **Promps-Ent** — ブロック志向のプロンプトジェネレータ
  - **ai2ia** — プロンプトジェネレータ + AI 比較ツール
  - **JaIM** — Linux 専用の日本語 Input Method
