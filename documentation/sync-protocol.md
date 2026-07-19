# Sync Protocol v0

## Scope

Every sync operation is scoped to two things:

1. **User**: carried in the `Authorization` header. Ignored by the v0 store, but every client request sends the header from day one so adding real auth later should be easy.
2. **Drive**: a named root directory that syncs as a unit (what git would call a repository and Obsidian a vault). It lives in the URL: `/api/drives/{driveId}/...`. We chose "drive" because non-technical users already know the concept from Google Drive. We avoided "folder" because `Node::Folder` already means something else in the codebase, and "repository" because we don't want to import git's expectations (branches, commits).

The two scopes split the data model into:

- **Nodes and the root are drive-scoped.** The root hash points at a node and nodes reference each other by hash, so the drive is build by traversing the node graph. They live under `/api/drives/{driveId}/...`.
- **Blobs are user-scoped.** A blob is pure content addressed by its hash, so the same content in two drives is the same blob, what that scoping blobs to the user we end up deduping across drives for free. They live under `/api/blobs/...`, with the user implied by the `Authorization` header.

We deliberately do not dedup blobs across users, even though a hosted deployment would save some storage with it. Cross-user dedup only pays off for widely shared content, and it is incompatible with end-to-end encryption since identical plaintexts encrypt to different ciphertexts under different user keys, and the known workaround (convergent encryption, where the key derives from the content) lets the store confirm whether a user possesses a specific file which is a privacy leak.

Drives will be implicit in v0, so writing to a driveId creates it if its not present yet.

All hashes are blake3, hex-encoded, computed by the client. Nodes and blobs are immutable and addressed only by hash, never by path (see [main.md](main.md) for why).

## Endpoints

All endpoints are mounted under `/api`.

| Method | Path                                 | Body          | Success                                              | Absent                          |
| ------ | ------------------------------------ | ------------- | ---------------------------------------------------- | ------------------------------- |
| GET    | `/api/drives/{driveId}/root`         | –             | 200, JSON                                            | 404 (root never set)            |
| GET    | `/api/drives/{driveId}/versions`     | –             | 200, JSON                                            | –                               |
| PUT    | `/api/drives/{driveId}/root`         | JSON          | 204                                                  | 409 (Compare-And-Swap mismatch) |
| GET    | `/api/drives/{driveId}/nodes/{hash}` | –             | 200, JSON node                                       | 404                             |
| PUT    | `/api/drives/{driveId}/nodes/{hash}` | JSON node     | 204                                                  | –                               |
| HEAD   | `/api/blobs/{hash}`                  | –             | 200                                                  | 404                             |
| POST   | `/api/blobs/{hash}/upload`           | JSON `{size}` | 200, JSON ticket (204 if the blob is already stored) | –                               |
| PUT    | `/api/blobs/{hash}/bytes`            | raw bytes     | 204                                                  | –                               |
| POST   | `/api/blobs/{hash}/commit`           | –             | 204                                                  | 404 (no ticket for this hash)   |
| POST   | `/api/blobs/{hash}/download`         | –             | 200, JSON ticket                                     | 404                             |
| GET    | `/api/blobs/{hash}/bytes`            | –             | 200, raw bytes                                       | 404                             |

Route segments (`drives`, `nodes`, `blobs`) stay plural to match REST convention. The store's router modules are named singular (`drive`, `node`, `blob`) since each module handles one resource type, not a collection of route segments.

### Status code semantics

These map onto the two error families the syncer already distinguishes in `SyncError`:

- **404 is maped as an answer not an error.** It maps to `Ok(None)` in the repository traits: "this repository does not have that hash". `compute_diff` probes the store with hashes it expects to be absent, so 404 is the common case.
- **5xx and network failures are transport errors.** They map to `NodeRepositoryError` / `BlobRepositoryError`, and the caller may retry. The syncer already retries naturally on the next watcher event.
- **409 on root PUT means lost race.** Another client moved the root. Re-scan and reconcile from scratch; DO NOT retry the same PUT.

All PUTs are idempotent thanks to the content being addressed by its own hash, so writing the same hash twice is a no-op.

### Root and compare-and-swap

`GET /root` returns:

```json
{ "hash": "c871c6fd84d8..." }
```

`PUT /root` sends the new root and the root the client believes is current, so two clients cannot silently stomp each other:

```json
{
  "hash": "<new root>",
  "expected": "<previous root, or null if the drive was empty>"
}
```

The store compares `expected` against its current root: match means flip and 204, mismatch means 409 with the actual current root in the body. The v0 store implements this check.

### Node wire format

JSON mirroring the `Node` enum in `crates/phaneros/src/node_repository/node.rs`:

```json
{ "type": "folder",
  "folders": [ { "name": "sub", "hash": "..." } ],
  "files":   [ { "name": "a.txt", "hash": "..." } ] }

{ "type": "file",
  "blobs": [ { "hash": "...", "size": 5885 } ] }
```

Entries stay sorted by name, as `Node::folder()` already guarantees. The hash is computed over that canonical order, so serialization must not reorder them.

Blobs are raw bytes (`application/octet-stream`).

### Blob transfer is divided into getting a ticket and moving the bytes

Blob upload and download go through the control plane only to obtain a **ticket**. The final bytes themselves move against the ticket's URL. This keeps the door open for serving blobs from a data plane that is not the control-plane store (S3/R2, or a storage service deployed alongside, per main.md) without changing the client.

A ticket is:

```json
{ "url": "<where to send/fetch the bytes>", "expires_at": <unix timestamp or null> }
```

Because the bytes may bypass the control plane entirely (going straight to S3/R2), a blob has two states in the metadata plane: **declared** (a ticket was minted, size known, bytes not yet stored) and **committed** (bytes stored). Only committed blobs are "held". Upload is therefore three steps:

- **Upload ticket**: `POST /api/blobs/{hash}/upload` with `{ "size": <bytes> }`. The client declares the size here because it is the only point the control plane is guaranteed to see it (the bytes may never touch the control plane). This is also where the size the store records for pruning is captured, and what the ticket enforces. 200 returns a ticket and marks the blob declared; 204 means the store already holds the blob and the client skips the upload (a second dedup guard besides `compute_diff`'s HEAD probe).
- **Move bytes**: the client PUTs the raw bytes to `ticket.url`. The transfer enforces the declared size. This step does **not** commit the blob.
- **Commit**: `POST /api/blobs/{hash}/commit`. The client confirms the bytes have landed, and the store flips the blob to committed. This is the sole committer, so the flow is identical whether the bytes went to this store or to R2. 404 means no ticket was ever minted for this hash.
- **Download**: `POST /api/blobs/{hash}/download`. 200 returns a ticket and the client GETs the bytes from `ticket.url`; 404 means the blob is absent, the usual `Ok(None)`.

The client must treat `url` as opaque. In v0 the store mints URLs pointing at itself (`/api/blobs/{hash}/bytes` on the same host) and `expires_at` is null . Later the same field carries a presigned S3/R2 URL with a real expiry, and an expired ticket simply gets re-requested.

`HEAD /api/blobs/{hash}` stays on the control plane since its a reponse about metadata, not the bytes themselves.

### Encryption readiness

E2EE is a later addition, but the protocol accepts it as of now, because the store treats content as opaque and only ever interprets the hash graph:

- **Blob bytes are opaque already.** An encrypting client uploads ciphertext, so nothing on the wire changes. Hashes are always computed by the client over the bytes it uploads, so under E2EE they are hashes of ciphertext. Plaintext hashes must never appear on the wire, since they would let the store confirm file contents by hash.
- **Node `name` fields are opaque strings.** The store never interprets names, so an encrypting client can put ciphertext there. `Node::folder()` hashes whatever bytes the names contain, so the hashing scheme is unaffected. One client-side constraint follows: name encryption must be deterministic, otherwise every re-scan would produce different node hashes for unchanged folders and nothing would ever dedup.
- **The hash graph stays plaintext.** The store needs `hash` fields readable to walk trees for GC's mark phase. This is the one part of a node the store must understand, but its safe since it reveals nothing about content.

The accepted metadata leaks under E2EE are: tree shape, blob sizes, and update timing remain visible to the store. Key management is entirely client-side and out of protocol scope.

### Versions

Nodes never have versions, only the root pointer does. Every node is immutable, so a "version" of a file is just a different node reachable from an older root. The store therefore records exactly one thing: the sequence of root hashes as they flip. On every accepted root PUT it appends `{root, at}` to the drive's version log.

`GET /api/drives/{driveId}/versions` returns that log, newest first (empty list if the drive has no history yet):

```json
{ "versions": [{ "root": "<hash>", "at": <unix timestamp> }] }
```

Reading a version needs no other endpoint, since the client can take an old root hash and walks it with the existing `GET /api/drives/{driveId}/nodes/{hash}`, which works because retained nodes stay addressable by hash after they become unreachable from the current root.

Per-file history ("versions of `docs/thesis.md`") is derived client-side. The client should walk the version roots, resolve the path in each, and collect the distinct node hashes. This is forced by E2EE, since resolving a path means reading names, and under E2EE the store cannot do that.

## Invariants

The write ordering the client must follow is very important to maintain the store as simple as possible:

1. **Blobs, then nodes, and root last.** A node PUT should imply its blobs are already on the store; the root PUT should imply the whole tree is. The store never has to validate reachability on write, and its visible tree (whatever the root points at) is never dangling.
2. **Orphans are fine.** A client that crashes mid-upload leaves unreachable nodes/blobs. That is the GC's job to clean up.
3. **The store trusts hashes in v0.** It stores what the client sends under the hash the client names. Verifying blob content against its hash is cheap and worth adding, but it is an integrity feature not a requirement for the protocol to function.

## Deferred (known, intentionally not in v0)

- **Batch negotiation**: `compute_diff` probes one hash per request, which means N round trips. The fix is a `POST .../missing` endpoint taking a hash list and returning the subset the store lacks, one per scope (`/api/drives/{driveId}/nodes/missing` and `/api/blobs/missing`).
- **Auth**: JWT in the `Authorization` header, per main.md.
- **E2EE**: passphrase-derived keys in the client/daemon, encrypting blob bytes and node names.
- **Change notifications**: SSE from store to client, per main.md. v0 is push-only from the watching client.
- **External data plane**: the ticket flow is already in the protocol, so what is deferred is the store minting URLs that point somewhere else (presigned S3/R2) and enforcing `expires_at`. v0 serves the bytes itself.
- **Drive management**: create/list/delete drives.

## Open questions

1. Does `driveId` come from the store (client registers, gets an id) or the client (a name/uuid the client picks)? v0 uses whatever string the client puts in the URL.
