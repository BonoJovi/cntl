# DR-003: Object Encoding — postcard over bincode

# DR-003: オブジェクトエンコーディング — bincode から postcard へ

**Status / 状態**: Accepted (pre-v0.3.0)

---

## Context / 背景

**EN.** Every stored object (blob / tree / commit) is serialized with `object::encode()` before being hashed and written to the `objects` table. Critically, `hash_bytes()` is applied to the **encoded bytes**, not to the object's logical content:

```rust
let bytes = object::encode(&blob)?;
let hash = object::hash_bytes(&bytes);
object::store_object(conn, &hash, "blob", &bytes)?;
```

This means the serialization library is not a swappable implementation detail — it is part of the on-disk object format. Any change to how the same logical value is encoded changes every hash derived from it. Since v0.2.0 has already shipped, this is a format-compatibility decision, not a routine dependency bump.

`bincode` 2.0.1 was flagged by `cargo audit` as unmaintained (RUSTSEC-2025-0141) following a doxxing and harassment incident that ended the project's development. The advisory carries no known vulnerability — it is a maintenance-status warning, not a security defect.

**JA.** 保存される各オブジェクト (blob / tree / commit) は `object::encode()` でシリアライズされたのち、その結果に対してハッシュを計算し `objects` テーブルへ書き込まれる。重要なのは、`hash_bytes()` がオブジェクトの論理的な内容ではなく**エンコード後のバイト列**に対して適用される点である:

```rust
let bytes = object::encode(&blob)?;
let hash = object::hash_bytes(&bytes);
object::store_object(conn, &hash, "blob", &bytes)?;
```

つまりシリアライズライブラリは差し替え可能な実装詳細ではなく、オンディスクのオブジェクトフォーマットそのものの一部である。同じ論理的な値であっても、エンコード方式が変わればそこから導かれるハッシュはすべて変わる。v0.2.0 が既にリリース済みである以上、これは通常の依存更新ではなく、フォーマット互換性に関わる判断である。

`bincode` 2.0.1 は `cargo audit` によって unmaintained と検出された (RUSTSEC-2025-0141)。開発者へのdoxxing被害により開発チームが開発終了を決定したことによるもので、既知の脆弱性はない。セキュリティ上の欠陥ではなく、メンテナンス状態の警告である。

---

## Decision / 判断

**EN.** Replace `bincode` with [`postcard`](https://postcard.jamesmunns.com) for all object encoding, ahead of v0.3.0 (merge feature).

Two factors drove the timing and the choice:

1. **Timing**: cntl is still pre-merge / pre-conflict-resolution. No object format built on top of the current encoding yet exists in the roadmap. This is the cheapest point at which to absorb a hash-affecting change — every version after v0.3.0 adds more surface area (merge commits, conflict metadata) that would also need to carry the migration.
2. **Library choice**: `postcard` publishes a formally specified, versioned wire format as a primary design goal (born from `no_std`/embedded use, where wire stability across toolchains matters as much as it does here). This matches cntl's requirement that object hashes remain reproducible indefinitely, better than an actively-evolving but format-unspecified crate would.

**JA.** v0.3.0 (merge 機能) に着手する前に、全オブジェクトのエンコーディングを `bincode` から [`postcard`](https://postcard.jamesmunns.com) に置き換える。

タイミングとライブラリ選定の判断理由は2点:

1. **タイミング**: cntl はまだ merge / conflict 解消に未着手。現行エンコーディングの上に構築されたオブジェクトフォーマットはロードマップ上まだ存在しない。ハッシュに影響する変更を吸収するには今が最も安いタイミングである — v0.3.0 以降は machine (merge commit, conflict metadata 等) が積み上がるほど、移行時に一緒に運ぶ荷物が増える。
2. **ライブラリ選定**: `postcard` は「仕様化・バージョン管理されたワイヤーフォーマット」を第一の設計目標として掲げている (`no_std` / 組み込み用途由来で、ツールチェーンをまたいだワイヤー安定性が同様に重視される文脈で育った)。「オブジェクトハッシュが将来にわたって再現可能であること」という cntl の要求と、活発だがフォーマット仕様を明示しないクレートよりも噛み合う。

---

## Alternatives Considered / 検討した代替案

### A. Stay on bincode 2.0.1, silence the audit warning / bincode 2.0.1 を維持し audit 警告を抑制

**EN.** Viable but deferred, not rejected outright. `bincode` 2.0.1 has no known vulnerability; the advisory itself states 1.3.3 (an earlier line) is "complete" and needs no further work. Doing nothing was defensible. It was set aside because a format-affecting migration will very likely become necessary eventually (unmaintained crates accumulate risk over time even without an active CVE today), and pre-v0.3.0 is the cheapest point to take that cost — waiting only makes the eventual migration more expensive.

**JA.** 選択肢としては成立するが、今回は保留ではなく明示的に見送った。`bincode` 2.0.1 に既知の脆弱性はなく、advisory 自体も (旧系列である) 1.3.3 は「完成されており追加対応不要」としている。何もしないという判断も擁護可能ではあった。それでも見送ったのは、メンテナンスが止まったクレートは今日 CVE がなくてもリスクが時間とともに蓄積するため、いずれフォーマットに影響する移行が必要になる可能性が高く、v0.3.0 着手前が最も安いコストでその移行を実行できるタイミングだからである。待てば待つほど、いずれ来る移行のコストが上がる。

### B. bitcode / rkyv

**EN.** Rejected for this decision. `bitcode` prioritizes speed/size over a documented stable wire format across versions; `rkyv`'s zero-copy archive model requires deeper type-level changes (`Archive` derive, alignment concerns) for a benefit (zero-copy reads) that cntl's SQLite-backed, per-object access pattern does not need. Both remain worth reconsidering if a future performance bottleneck specifically implicates encoding cost.

**JA.** 今回の判断としては見送り。`bitcode` はバージョン間でのワイヤーフォーマット安定性の文書化よりも速度・サイズを優先している。`rkyv` のゼロコピーアーカイブモデルは型レベルでの変更 (`Archive` derive、アラインメント考慮) を要求する一方、その利点 (ゼロコピー読み取り) は cntl の SQLite 経由・オブジェクト単位アクセスというパターンでは必要性が薄い。将来エンコーディングコストが具体的にボトルネックとして特定された場合には、両者とも再検討の余地がある。

---

## Consequences / 結果

**EN.**

- All object hashes generated before this change are incompatible with hashes generated after it — any pre-existing `.cntl/repo.db` created before this commit will not interoperate with builds after it. Given the small number of repositories created so far (`cargo install` distribution has not yet begun — see release strategy), this is treated as an acceptable one-time break rather than a migration requiring tooling.
- `object::encode()` / `object::decode()` keep the same signatures; only their internals changed, so all call sites in `cli.rs` were unaffected.
- Going forward, `postcard`'s documented format spec is the reference for anything that needs to reason about raw object bytes outside of this codebase (recovery tooling, external inspection, etc.).

**JA.**

- この変更より前に生成されたオブジェクトハッシュは、変更後のハッシュと非互換になる。この commit 以前に作られた `.cntl/repo.db` は、以降のビルドと相互運用できない。これまでに作られたリポジトリ数は少なく (`cargo install` での配布はまだ開始していない — release strategy 参照)、移行ツールを要する話ではなく一度きりの許容可能な破壊的変更として扱う。
- `object::encode()` / `object::decode()` はシグネチャを変えていないため、`cli.rs` 側の呼び出し箇所への影響はない。
- 今後、生のオブジェクトバイト列を cntl 本体の外で扱う必要が生じた場合 (復旧ツール、外部からの検査等) は、`postcard` の仕様化されたフォーマットドキュメントが参照先になる。

**EN.** `postcard`'s `default` feature set (`heapless-cas`) pulls in `heapless` for `no_std` embedded targets, which transitively depends on the unmaintained `atomic-polyfill` (RUSTSEC-2023-0089) — this would have reintroduced the same class of warning we were trying to remove. cntl uses only `postcard::to_allocvec` / `from_bytes`, so the dependency is declared with `default-features = false, features = ["alloc"]`, which drops the entire `heapless` chain. `cargo audit` is clean after this trim.

**JA.** `postcard` のデフォルト機能セット (`heapless-cas`) は `no_std` 組み込みターゲット向けの `heapless` を引き込み、そこから未メンテナンスの `atomic-polyfill` (RUSTSEC-2023-0089) へ推移的に依存する — これでは今回取り除こうとしていたのと同種の警告を再導入することになる。cntl は `postcard::to_allocvec` / `from_bytes` しか使わないため、依存宣言を `default-features = false, features = ["alloc"]` とし、`heapless` 系列を丸ごと除外した。この絞り込み後、`cargo audit` は警告ゼロになる。

---

## Related / 関連

- [DR-001](DR-001-storage.md) — storage design this encoding feeds into / このエンコーディングが書き込まれるストレージ設計
- [README](../../README.md) — overview & roadmap / 全体像とロードマップ
- [CONCEPT.md](../CONCEPT.md) — object model & tech stack / オブジェクトモデルと技術スタック
