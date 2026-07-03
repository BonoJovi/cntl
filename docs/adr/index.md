# Design Rationale Records (Index) / 設計判断記録 (索引)

This directory collects **Design Rationale (DR)** records — the *why* behind specific implementation choices in cntl.

このディレクトリには、cntl の具体的な実装判断について「**なぜそう作るか**」を記録した **Design Rationale (DR)** ドキュメントを置きます。

- See [README](../../README.md) for an overview and roadmap.
- See [CONCEPT.md](../CONCEPT.md) for the conceptual design ("what cntl is").
- DR documents below cover the rationale ("why we made each choice").

- 全体像とロードマップは [README](../../README.md) を参照。
- 概念設計 (cntl が何を目指すか) は [CONCEPT.md](../CONCEPT.md) を参照。
- 以下の DR ドキュメントは個別の判断 (なぜそう作るか) を扱います。

---

## Index / 索引

| ID | Title / タイトル | Status / 状態 |
|---|---|---|
| [DR-001](DR-001-storage.md) | Storage Design / ストレージ設計 — Single SQLite per repository / 1 リポジトリ = 1 SQLite ファイル | Accepted (v0.1.0) |
| [DR-002](DR-002-head-and-branches.md) | HEAD and Branch Model / HEAD とブランチのモデル — Symbolic HEAD with no accidental detachment / シンボリック HEAD と事故的 detached の排除 | Accepted (v0.2.0) |
| [DR-003](DR-003-object-encoding.md) | Object Encoding / オブジェクトエンコーディング — postcard over bincode / bincode から postcard へ | Accepted (pre-v0.3.0) |

---

## Conventions / 表記規約

- One file per decision: `DR-NNN-<slug>.md`
- Bilingual: English and Japanese in parallel, per section
- Status values: `Proposed` / `Accepted` / `Superseded by DR-NNN` / `Deprecated`

- 1 判断につき 1 ファイル: `DR-NNN-<スラッグ>.md`
- 英日併記: 各セクションで両言語を並置
- ステータス: `Proposed` / `Accepted` / `Superseded by DR-NNN` / `Deprecated`
