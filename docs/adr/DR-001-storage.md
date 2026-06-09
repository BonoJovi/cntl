# DR-001: Storage Design — Single SQLite per Repository

# DR-001: ストレージ設計 — 1 リポジトリ = 1 SQLite ファイル

**Status / 状態**: Accepted (v0.1.0)

---

## Context / 背景

**EN.** During exploration of how a Rust-based VCS should store its data, we hit a recurring practical frustration with Git: copying a repository wholesale with `cp -r .git/` frequently stalls or slows dramatically on individual files inside `.git/objects/`. This is a known consequence of filesystems being asymmetric about "few large files" vs "many small files":

- inode lookup and metadata operations dominate on large directory trees
- network filesystems pay per-file round-trip costs
- interrupted operations leave partially-copied state that is hard to recover

**JA.** Rust 製 VCS をどう設計するか検討する過程で、Git の実運用上の摩擦点として繰り返し直面したのが、`cp -r .git/` でリポジトリを丸ごとコピーしようとすると `.git/objects/` の中の 1 ファイルでしばしば停止・極端に遅延する現象だった。これはファイルシステムが「少数の大きなファイル」と「多数の小さなファイル」に対して非対称なコストを持つことに由来する:

- 大規模なディレクトリツリーでは inode 探索とメタデータ操作のコストが支配的
- ネットワーク FS では per-file の往復コストが積算
- 中断されたコピーは中途半端な状態を残し、復旧が難しい

**EN.** The deeper observation: *a system whose preservation step can stall on file-state is not a complete system.* Data preservation should not require the user to think about individual files at all.

**JA.** より深い観察として、**保全のステップがファイルステートで止まりうるシステムは、システムとして完璧ではない**。データ保全に際してユーザが個別のファイルを意識しなければならない状態自体が、設計の不足である。

---

## Decision / 判断

**EN.** Store all per-repository data — content-addressable objects (blob / tree / commit), refs (HEAD, future branches/tags), and repository-local settings — in **a single SQLite file** at `.cntl/repo.db`, placed inside the project working directory.

Each repository has its own SQLite file. There is no centralized, system-wide aggregation of repository data.

**JA.** リポジトリ単位の全データ — content-addressable オブジェクト (blob / tree / commit)、refs (HEAD、将来のブランチ/タグ)、リポジトリローカルの設定 — を**単一の SQLite ファイル** `.cntl/repo.db` に格納する。このファイルはプロジェクトの作業ディレクトリ内に置く。

各リポジトリは独立した SQLite ファイルを持つ。リポジトリデータを一元集約する仕組みは持たない。

---

## Alternatives Considered / 検討した代替案

### A. Git-style: Many small files under `.cntl/objects/` / Git 方式: 多数の小ファイル

**EN.** Rejected. This is precisely the design that produced the original pain point. While the content-addressable model is elegant in theory, exposing it as a sprawling directory tree concentrates filesystem-level fragility right where users will most need it to work (backup, copy, sync).

**JA.** 棄却。これこそが起点となった痛みを生み出している設計。content-addressable モデルは理論的には美しいが、それを大規模なディレクトリツリーとして表に出すと、ユーザが最も機能してほしい場面 (バックアップ / コピー / 同期) でファイルシステムレベルの脆さが集中することになる。

### B. Single global SQLite for all repositories / 全リポジトリを 1 つのグローバル SQLite に集約

**EN.** Rejected. See [Boundary Conditions](#boundary-conditions--境界条件) below — the "single file" idea has explicit limits when scaled to all-repositories.

**JA.** 棄却。理由は後述の [境界条件](#boundary-conditions--境界条件) を参照。「1 ファイル」の考え方には、全リポジトリスケールに引き上げた場合の明確な限界がある。

---

## Consequences / 結果

**EN.**

- **Backup, copy, migration**: a single file to handle. `cp .cntl/repo.db backup.db` is the entire backup story for a repository's VCS state.
- **Atomicity is natural**: SQLite's transaction model maps directly onto cntl's commit semantics. A `tx.commit()` makes blob / tree / commit / HEAD updates atomic — no possibility of half-written state on the filesystem.
- **Clean lifecycle boundaries**: deleting the project directory deletes all VCS data; no orphaned objects, refs, or config can be left behind on disk.
- **Predictable performance**: filesystem cost is dominated by one file's size, not by file count.

**JA.**

- **バックアップ・コピー・移行**: 1 ファイルだけ扱えば済む。`cp .cntl/repo.db backup.db` がそのままバックアップ手順になる。
- **atomicity が自然に得られる**: SQLite のトランザクションモデルが cntl のコミット意味論にそのまま対応する。`tx.commit()` で blob / tree / commit / HEAD の更新が atomic になり、ファイルシステム上に中途半端な書き込み状態が残らない。
- **ライフサイクル境界が明確**: プロジェクトディレクトリを削除すれば VCS データもすべて消える。孤児になったオブジェクト・refs・設定がディスク上に残らない。
- **パフォーマンスの予測可能性**: ファイルシステムコストは「1 ファイルのサイズ」で支配される。ファイル数では支配されない。

---

## Boundary Conditions / 境界条件

**EN.** "Single file" is not an absolute principle — it has explicit limits.

**JA.** 「1 ファイル原則」は絶対ではなく、明確な境界がある。

### Axis 1: Corruption risk scales with file size / 軸 1: 破損リスクのスケール則

**EN.** SQLite files become more vulnerable to corruption (physical media errors, partial writes, journaling corner cases) as they grow. Aggregating all repositories into one global file means:

- A single corruption event can destroy *every* project.
- Per-repository files cap the blast radius at one project.

This is why a system-wide single file was rejected: the "single file" principle, scaled up, *inverts* its own reliability advantage.

**JA.** SQLite はサイズが大きくなるほど破損リスク (物理破損、部分書き込み、ジャーナリングのコーナーケース等) に脆くなる。全リポジトリを 1 つのグローバルファイルに集約すると:

- 1 回の破損で**すべて**のプロジェクトが死ぬ可能性がある。
- リポジトリ単位なら、blast radius は最悪でも 1 プロジェクトに制限される。

「1 ファイル原則」をユーザ全体スケールに引き上げると、信頼性のスケール則が**逆転する**。これがグローバル 1 ファイル案を棄却した第一の理由。

### Axis 2: Human memory is unreliable / 軸 2: 人間の記憶力は当てにならない

**EN.** A centralized database forces the user to remember where the data lives. Per-repository placement removes that cognitive burden:

| Aspect | Global DB | Per-repo DB |
|---|---|---|
| Locate the data | "Where did I put `cntl/all.db`?" | "It's in the project folder." |
| Identify a project's data | Inspect the DB | "That folder, by definition." |
| Move a project | Move project + remember to migrate DB rows | `mv project/` (data follows) |
| Delete a project | Risk of orphaned rows | `rm -rf project/` (clean) |
| Back up a project | Extract specific rows | Back up the project folder |

**JA.** 一元化された DB は、ユーザに「どこに置いたか」を覚えさせる。リポジトリ単位の配置はこの認知負荷を排除する:

| 項目 | グローバル DB | リポジトリ単位 DB |
|---|---|---|
| データの場所 | 「`cntl/all.db` どこ置いたっけ?」 | 「プロジェクトフォルダ内」 |
| プロジェクトのデータ識別 | DB の中身を見る必要 | 「そのフォルダ、定義上」 |
| プロジェクト移動 | プロジェクト + DB 行の移行 | `mv project/` (データも追従) |
| プロジェクト削除 | 孤児行が残るリスク | `rm -rf project/` (綺麗に消える) |
| プロジェクトのバックアップ | 特定行を抽出 | プロジェクトフォルダごとバックアップ |

**EN.** The design principle: **shape the system to fit human intuition, rather than push cognitive load onto the user.**

**JA.** ここで採用している設計原則: **構造を人間の直観に寄せる。人間の認知負荷を構造に押し付けない。**

---

## Exception: Global Config (`global.db`) / 例外: グローバル設定 (`global.db`)

**EN.** User-level configuration (e.g., `user.name`, `user.email`) is stored at `~/.config/cntl/global.db`, not in any repository. This exception is justified by:

- **Small size**: corruption risk and blast radius are both small.
- **Content is genuinely user-global**: not bound to any one repository.
- **Discoverable by convention**: XDG `$XDG_CONFIG_HOME` makes the location predictable for any user who knows the conventions of their OS.

This narrow exception confirms, rather than contradicts, the boundary conditions above: storing config globally is acceptable *because* it doesn't trigger either axis (small data, location follows a known convention).

**JA.** ユーザ単位の設定 (`user.name`, `user.email` 等) は `~/.config/cntl/global.db` に置く。これは境界条件の例外にあたるが、以下の理由で正当化される:

- **データサイズが小さい**: 破損リスクも blast radius も小さい。
- **本質的にユーザグローバル**: 特定のリポジトリに紐づかない。
- **規約により発見可能**: XDG `$XDG_CONFIG_HOME` 規約に従うため、OS の規約を知っているユーザなら場所を予測できる。

この限定的な例外は境界条件と矛盾するのではなく、むしろ強化する: 設定をグローバルに置くことが許容されるのは、**両軸 (破損スケール則 / 認知負荷) をどちらも発火させないから**である。

---

## "Just-Right Granularity" Principle / 「ちょうどの粒度」原則

**EN.** Three points on the same axis:

| Design | Granularity | Problem |
|---|---|---|
| Git | 1 repo = many object files | Internal complexity exposed to filesystem |
| (Rejected) Global SQLite | All repos = 1 file | Over-hidden; risk concentrated |
| cntl | **1 repo = 1 SQLite file** | System unit matches human cognitive unit |

The choice is not "make it one file" — it is "**make the file boundary match the unit the user already thinks in**."

**JA.** 同じ軸上の 3 点として整理する:

| 設計 | 粒度 | 問題 |
|---|---|---|
| Git | 1 リポ = 多数の objects ファイル | 内部複雑性がファイルシステムに露出 |
| (棄却案) グローバル SQLite | 全リポ = 1 ファイル | 隠蔽過多、リスク集中 |
| cntl | **1 リポ = 1 SQLite ファイル** | システムの単位 = 人間の認知単位 |

判断の本質は「1 ファイルにする」ではなく、「**ファイル境界を、ユーザが既に持っている認知単位に一致させる**」ことにある。

---

## Related / 関連

- [README](../../README.md) — overview & roadmap / 全体像とロードマップ
- [CONCEPT.md](../CONCEPT.md) — vision and conceptual model / ビジョンと概念モデル
- [TUTORIAL.md](../TUTORIAL.md) — hands-on walkthrough / 操作チュートリアル
