# DR-002: HEAD and Branch Model — Symbolic HEAD with No Accidental Detachment

# DR-002: HEAD とブランチのモデル — シンボリック HEAD と「事故的 detached」の排除

**Status / 状態**: Accepted (v0.2.0)

---

## Context / 背景

**EN.** v0.1.0 stored a single ref named `HEAD` whose `target` was a commit hash. With no branches, this was sufficient: `HEAD` was the only thing that could move, and it always pointed at "the current commit." v0.2.0 introduces branches, which forces three intertwined decisions:

1. What does `HEAD` actually point at — a commit, or a branch?
2. When does the default branch (`main`) come into existence?
3. What happens when the user wants to look at an arbitrary commit that no branch points at (Git's *detached HEAD* state)?

These three questions cannot be answered independently; any answer to one constrains the others.

**JA.** v0.1.0 では `HEAD` という名前の単一の ref があり、その `target` は commit hash だった。ブランチが存在しないうちは、これで十分だった: 動くものは `HEAD` だけで、それは常に「いまの commit」を指していた。v0.2.0 でブランチを導入するにあたり、互いに絡み合う 3 つの判断が必要になる:

1. `HEAD` は何を指すのか — commit か、それともブランチか?
2. デフォルトブランチ (`main`) はいつ生まれるのか?
3. 「どのブランチからも指されていない任意の commit を見たい」場合 (Git の *detached HEAD* 状態) はどう扱うのか?

この 3 つは独立に決められない。1 つを決めると他の答えが制約される。

**EN.** Git's answers — symbolic `HEAD`, lazy `main` creation at first commit, freely-reachable detached state — are the de facto reference point, but the third has a long history of user confusion ("I committed and now my work is gone"). cntl's design principles ("reduce friction", "don't corrupt the working file") push back specifically on that third item.

**JA.** Git の答え — シンボリック `HEAD`、`main` は first commit で遅延作成、detached 状態は誰でも入れる — は事実上の参照点だが、3 番目はユーザを混乱させてきた長い歴史を持つ (「コミットしたのに作業が消えた」)。cntl の設計原則 (「摩擦を減らす」「作業ファイルを壊さない」) は、この 3 番目に対して特に押し戻したい。

---

## Decision / 判断

**EN.** Three coupled decisions.

**JA.** 3 つの結合した判断:

### D1. `HEAD` is a symbolic reference / `HEAD` はシンボリック参照

**EN.** `HEAD` does not store a commit hash. It stores the name of a branch ref (e.g. `refs/heads/main`). The commit is resolved by following one extra hop: `HEAD → branch ref → commit hash`.

**JA.** `HEAD` は commit hash を保持しない。ブランチ ref の名前 (例: `refs/heads/main`) を保持する。commit を取得するときは 1 段余分にたどる: `HEAD → ブランチ ref → commit hash`。

### D2. The default branch is created lazily at the first commit / デフォルトブランチは first commit で遅延作成

**EN.** Immediately after `cntl init`, `HEAD` points at `refs/heads/main`, but `refs/heads/main` itself does not yet exist. The first `cntl commit` creates the branch ref and the commit atomically in one transaction. Subsequent commits update the branch ref in place.

**JA.** `cntl init` 直後は `HEAD` が `refs/heads/main` を指しているが、`refs/heads/main` 自体はまだ存在しない。最初の `cntl commit` で、ブランチ ref と commit を 1 トランザクションで同時に生成する。以降の commit はそのブランチ ref を更新するだけ。

### D3. Detached HEAD is unreachable from the normal mode / 通常モードからは detached HEAD に到達できない

**EN.** In normal operation, `HEAD` is *always* a symbolic ref pointing at some branch. There is no command surface in normal mode that detaches `HEAD` from a branch — no `cntl checkout <commit-hash>`, no implicit detachment from operations like `cntl restore`. The invariant **"HEAD always points at a branch"** holds unconditionally outside of an explicit mode.

A separate mode (working name `inspect`, to be specified in a future DR) is the **only** way to view an arbitrary commit that no branch points at. Entering and leaving the mode is explicit on both sides.

**JA.** 通常運用では、`HEAD` は**常に**何らかのブランチを指すシンボリック参照である。通常モードのコマンド面に `HEAD` をブランチから外す手段は存在しない — `cntl checkout <commit-hash>` も無いし、`cntl restore` 等の副作用でうっかり外れることも無い。**「HEAD は常にブランチを指す」** という不変条件は、明示的モードの外では無条件に成立する。

任意 commit を見る手段は専用モード (仮称 `inspect`、詳細は将来の DR で規定) のみ。モードへの出入りは両方向で明示的。

---

## Alternatives Considered / 検討した代替案

### A1. `HEAD` stores a commit hash directly / `HEAD` が commit hash を直接保持

**EN.** Rejected. Considered because it requires no schema change from v0.1.0. But it forces every operation that "is on a branch" to maintain a separate `current_branch` setting, and to keep `HEAD` and `current_branch` in lockstep. Two pieces of state that *must* agree is a recurring source of bugs (and the symbolic-ref design exists precisely because Git went through this evolution itself). The savings on schema were not worth the ambient correctness cost.

**JA.** 棄却。v0.1.0 からスキーマ変更が不要なので検討した。しかし、「現在のブランチ」を別キー `current_branch` で持ち、`HEAD` と `current_branch` を常に同期させる必要が出る。**必ず一致させなければならない 2 状態**は典型的なバグの温床 (Git 自身もこの進化を経てシンボリック ref に至った)。スキーマで節約しても、恒常的な整合性コストに見合わない。

### A2. Create `main` eagerly at `cntl init` time / `init` 時に `main` を予約作成

**EN.** Rejected. To make `refs/heads/main` exist before any commit, the schema would need to allow ref targets to be "empty" or "null", introducing a special case that propagates into every consumer of the refs table. Lazy creation keeps the invariant "**a branch ref always points at a real commit**" intact, which is simpler to reason about everywhere downstream.

**JA.** 棄却。commit がまだ無い時点で `refs/heads/main` を実在させるには、ref の target に「空」や「null」を許す必要があり、refs テーブルを参照する全コードに特例が伝播する。遅延作成にすれば、**「ブランチ ref は常に実在する commit を指す」**という不変条件が壊れず、下流のロジックがすべて単純になる。

### A3. Allow Git-style freely-reachable detached HEAD / Git 風の自由な detached HEAD を許容

**EN.** Rejected. The trade-off is genuine — Git's detached HEAD is powerful, and the cost is "users sometimes lose work and don't know why." cntl explicitly chooses to *prevent the loss-of-work scenario by construction*, accepting a slight loss of expressiveness in the default surface and recovering it through an explicit mode.

This matches the existing CONCEPT.md design line: conflict resolution is also pushed behind an explicit mode (`cntl mode resolve`) rather than smeared into the default state. Mode is already a first-class concept in cntl; detached-commit inspection naturally belongs there.

The specifics of the inspection mode (its name, the entry/exit commands, what operations are allowed inside it) are deliberately *not* fixed here. They are deferred to a future DR once the v0.2.0 normal-mode surface (branch / checkout) is stable.

**JA.** 棄却。トレードオフは本物 — Git の detached HEAD は強力で、その代償は「ユーザがときどき作業を失い、なぜか分からない」状態である。cntl は**「作業喪失シナリオを構造的に起こさない」**ことを明確に選ぶ。デフォルト面の表現力は若干下がるが、それは明示モードで取り戻す。

これは CONCEPT.md の既存設計線とも整合する: conflict 解消も既存設計では明示モード (`cntl mode resolve`) に追いやられており、デフォルト状態に滲ませていない。cntl では既に「モード」が第一級の概念であり、detached commit の閲覧もそこに属するのが自然。

inspect モードの具体 (名前、出入りコマンド、内部で許可される操作) は本 DR では**意図的に確定しない**。v0.2.0 の通常モード面 (branch / checkout) が安定した後の将来 DR に委ねる。

---

## Consequences / 結果

**EN.**

- **Refs table semantics expand.** v0.1.0 stored only commit-hash targets in the `refs` table. v0.2.0 must accommodate symbolic targets (`HEAD → refs/heads/main`) alongside direct ones (`refs/heads/main → <commit hash>`). The exact schema shape — adding a `kind` column, splitting `HEAD` into its own table, or another encoding — is an implementation choice, not a design choice; it will be made when implementing and documented in code.
- **First-commit logic gains one step.** `cntl commit` now resolves `HEAD → refs/heads/main`, notices that the branch ref does not exist yet, and creates it (in the same transaction as the commit). Subsequent commits update the existing branch ref.
- **`cntl log` / `cntl status` / `cntl restore` change in one place only.** Each "read the current commit" call site is replaced with a single helper that does the symbolic-ref hop. No call site needs to know whether a branch exists yet — the helper returns `Option<ObjectHash>` exactly as before.
- **The current branch becomes observable.** `cntl branch` (no args) can list branches with the current one marked, because the current branch is literally what `HEAD` points at — no extra state required.
- **The inspection mode is now a known unknown.** The roadmap explicitly carries an open design item (entry/exit semantics, allowed operations) that will be resolved before any feature would need it.

**JA.**

- **refs テーブルの意味が広がる。** v0.1.0 は `refs` テーブルに commit hash の target だけを格納していた。v0.2.0 ではシンボリック target (`HEAD → refs/heads/main`) と直接 target (`refs/heads/main → <commit hash>`) を共存させる必要がある。スキーマの具体形 (`kind` カラム追加 / `HEAD` を別テーブル分離 / 別エンコード) は設計判断ではなく実装判断であり、実装時に決めてコード上に残す。
- **first commit のロジックに 1 段増える。** `cntl commit` は `HEAD → refs/heads/main` を解決した時点でブランチ ref がまだ無いことに気付き、それを commit と同じトランザクションで作成する。以降の commit は既存ブランチ ref を更新するだけ。
- **`cntl log` / `cntl status` / `cntl restore` の変更点は 1 箇所だけ。** 「現在の commit を読む」呼び出し箇所はすべて、シンボリック参照を 1 段たどるヘルパに置き換わる。呼び出し側はブランチ ref がまだ存在するかどうかを意識しなくてよい — ヘルパは従来どおり `Option<ObjectHash>` を返す。
- **現在ブランチが観測可能になる。** `cntl branch` (引数なし) は現在ブランチをマークしてブランチ一覧を出せる。現在ブランチとは `HEAD` がそのまま指しているものだから、追加状態は不要。
- **inspect モードが「既知の未確定事項」として登録される。** ロードマップに「入退出の意味論・許可される操作」という未確定設計項目が明示的に乗る。これを必要とする機能が来る前に解決する。

---

## Invariants / 不変条件

**EN.** After v0.2.0, the following hold in normal mode:

1. `HEAD` exists and is a symbolic ref. It always names a branch ref.
2. If a branch ref exists, it points at a commit object that exists in the `objects` table.
3. The branch ref named by `HEAD` may not yet exist (only between `cntl init` and the first commit). No other branch ref is ever in this "named but missing" state.
4. There is no working-tree operation, no commit operation, no restore operation that can leave `HEAD` pointing at a commit directly. The only path into that state is the (future) inspect mode.

**JA.** v0.2.0 以降、通常モードでは以下が成立する:

1. `HEAD` は必ず存在し、シンボリック参照である。常に何らかのブランチ ref を指す。
2. ブランチ ref が存在するなら、その指し先 commit は `objects` テーブルに必ず存在する。
3. `HEAD` が指すブランチ ref はまだ存在しない場合がある (`cntl init` から first commit までの間のみ)。それ以外のブランチ ref がこの「名指しされているが実在しない」状態に置かれることは無い。
4. 作業ツリー操作・commit 操作・restore 操作のいずれも、`HEAD` を直接 commit に張り替える経路を持たない。その状態に入る唯一の手段は将来の inspect モード。

---

## Related / 関連

- [DR-001](DR-001-storage.md) — Storage design (refs table origin) / ストレージ設計 (refs テーブルの出自)
- [CONCEPT.md](../CONCEPT.md) §3 — Mode-based conflict resolution (precedent for "mode as first-class concept") / モードベースの conflict 解消 (「モードを第一級概念とする」前例)
- README §Roadmap — v0.2.0 (branch / checkout / restore / diff) / v0.2.0 ロードマップ
