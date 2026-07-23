# Reference: Sync Plans and Three-Way Merge

## Sync plan derivation

`plan_sync(base, local, remote)` classifies the situation from three roots:

- `B` — base root: last converged canonical root, persisted locally.
- `L` — local root: current local scanned root.
- `R` — remote root: current remote store root (`None` if never set).

| Condition (checked in order)     | Plan                  |
| -------------------------------- | --------------------- |
| `B` is `None`                    | `RemoteBootstrapPull` |
| `R == L`                         | `Converged`           |
| `R` is `None`                    | `LocalPush`           |
| `B == L`, `R` differs            | `RemotePull`          |
| `B == R`, `L` differs            | `LocalPush`           |
| `L` and `R` both differ from `B` | `Merge`               |

`Merge` is the only plan where both sides diverged from base. The rest are fast-forwards and copy nodes/blobs in a single direction.

## Three-way merge

Merge is **recursive and keyed by entry name**, mirroring the node tree. For a merged folder, take the union of entry names across `base`, `local`, and `remote`. For each name, let `B`, `L`, `R` be the hash it maps to on each side (any may be **absent**). Apply the first matching rule:

### Per-name decision table

| Case                                              | Result                     | Reasoning                                         |
| ------------------------------------------------- | -------------------------- | ------------------------------------------------- |
| `L == R`                                          | keep `L`                   | Both sides agree (covers "both added the same").  |
| `B == L`, `R` differs (present)                   | take `R`                   | Local untouched → accept remote's change.         |
| `B == R`, `L` differs (present)                   | take `L`                   | Remote untouched → accept local's change.         |
| Absent in `B`, present on one side only           | keep that side             | It is a fresh **add**.                            |
| Present in `B`, `B == L`, absent in `R`           | drop it                    | Remote **deleted**, local untouched → honor it.   |
| Present in `B`, `B == R`, absent in `L`           | drop it                    | Local **deleted**, remote untouched → honor it.   |
| Both present, both differ from `B` and each other | **conflict** (see below)   | Modify/modify — needs subtree recursion or split. |
| Present in `B`, one side edited, other deleted    | **delete/modify conflict** | See policy below.                                 |

`base` is what disambiguates **add** (absent in base) from **delete** (present in base, absent on one side). Without it the two are indistinguishable.

### Conflict resolution

**Modify|modify on a folder.** Recurse: run the same merge on the three folder versions (`base`, `local`, `remote`). Do not pick a side, either side may hold adds/edits deeper in the subtree. The recursion terminates at equal hashes or at files.

**Modify|modify on a file.** The sync layer treats file contents as opaque blobs and does not understand formats, so no content-level (e.g. text) merge is done (we will evaluate in the future). Resolution is **keep both**: the merged folder gets two entries — `name → L` and `name.conflict → R`. This changes the folder's entry set and therefore its hash, which bubbles a new hash up to the root like any edit.

**Delete|modify.** One side deleted a file the other edited. There is no subtree to recurse into. Default policy: **preserve the edit** — keep the edited version, renamed `name.conflict-delete`, so an edit is never silently lost. Alternative policy (simpler, lossy) is **delete wins**, but it is not the default because it can silently lose edits. The user may later manually delete the `.conflict-delete` file if they want to honor the deletion.

### Building the merged tree

Because resolution can rename entries or pick different children, the merged tree is a **new tree with new hashes**, constructed bottom-up (a folder's hash depends on its children, so children must be resolved first). Every kept node's blobs must be present in the target blob repository; missing blobs are fetched from the source side before the node is written.

The merged root generally differs from **both** `L` and `R`. Reconciliation must therefore land the merged tree on both local and remote and point both roots at it, so the next `plan_sync` sees `R == L` → `Converged`. The new root becomes the next persisted `base`.

> **Why both sides must be written.** If `merge` wrote only local and set the local root to `M` (persisting `M` as base), the next `sync_once` would see `plan_sync(M, M, R)` → `RemotePull` (`B == L`, `R` differs), which would drag remote's _old_ tree `R` back over the merged tree and silently undo the merge. Writing both sides and setting both roots to `M` is what makes the next plan collapse to `Converged`.

### Implementation shape

Merge is recursive over folders via a helper that resolves one name at each level:

```rust
fn merge_folder(
    base_hash: Option<&Hash>,
    local_hash: Option<&Hash>,
    remote_hash: Option<&Hash>,
    plan: &mut MergePlan,
) -> Result<Option<Hash>, SyncError>
```

- Returns `Option<Hash>`: the identity the merged child resolved to, or `None`
  when nothing survives (fully deleted / empty). The parent uses this to build
  its own `Entry` list and, in turn, its own hash.
- Follows **collect-then-apply** so it never touches the repositories directly.
  Instead it threads a `&mut MergePlan` accumulator down the recursion and
  appends side effects into it. The top-level `merge` performs every write at the
  end. This keeps write ordering and abort-cleanliness in one place, and leaves
  room for batching writes to the HTTP store later.

Equal-hash short-circuits (`L == R`, `B == L`, …) avoid descending into subtrees
that are already resolved.

### `MergePlan`

The accumulator buckets side effects **by destination**, so the apply step knows
which repo to write to and which to read from. Constructed folders carry their
value (already in hand from `Node::folder`), so they need no fetch:

```rust
struct MergePlan {
    // sourced from remote repo, written to local
    to_local_nodes: HashSet<Hash>,
    to_local_blobs: HashSet<Hash>,
    // sourced from local repo, written to remote
    to_remote_nodes: HashSet<Hash>,
    to_remote_blobs: HashSet<Hash>,
    // freshly built merged folders — insert into BOTH sides
    built_nodes: Vec<(Hash, Node)>,
}
```

- A surviving subtree only `local` had → remote lacks it → `to_remote_*`.
- A surviving subtree only `remote` had → local lacks it → `to_local_*`.
- A newly constructed merged folder → both sides lack it → `built_nodes`.

### Apply order

The top-level `merge` drains the plan in a fixed order so a half-finished tree is
never visible:

1. **Blobs first:** `to_local_blobs` → local, `to_remote_blobs` → remote.
2. **Then nodes:** `to_local_nodes` → local, `to_remote_nodes` → remote, and
   `built_nodes` into both. Because folders are built bottom-up and pushed as
   they finish, `built_nodes` is already in child-before-parent order — insert in
   `Vec` order and no parent references a not-yet-inserted child.
3. **Roots last:** set local root to `M`, then remote root to `M`. Remote's
   `set_root` is a compare-and-swap against `R`; a lost race surfaces as
   `RootConflict` and aborts the reconcile with the visible tree still intact.
4. Return the number of nodes reconciled.
