# Phaneros Initial Documentation

> This document is to dump context and information about my development of Phaneros.

## What is Phaneros?

Phaneros is a aimed to be a simple and reliable way of synchronizing files between different devices and platforms.

## Architecture

The first design decision I had to make is the topology of the system. I decided to go with a traditional client-server topology instead of a peer-to-peer topology. This decision was made because I find the UX to be much simpler for a given user, specially a non-technical user. I didn't want the user to have to worry about setting up a peer-to-peer network, with its Relay and Discovery servers, and having to worry about the device being online or offline and why a file wasn't synchronized. I wanted the user to be able to just install the application, log in to a given server, and have their files synchronized without having to worry about the underlying network topology. This is separated from the fact of data ownership and privacy, which is an advantage of the peer-to-peer topology, but can be mitigated in a client-server topology with data encription and the option of self-hosting the server.

> How would authentication look like in a peer-to-peer topology?

Phaneros has two main components:

1. **Phaneros Client**: This is the application that runs on the device and is responsible for monitoring files and directories for changes, and communicating with the Phaneros Server to synchronize those changes.
2. **Phaneros Server**: This is the backend service that manages the synchronization process, storing the files and their versions, and handling requests from the Phaneros Client.

### Phaneros Client

#### Architecture

The phaneros client is composed of several components and follows a service oriented architecture. The main components are:

1. **Scanner**: This component is responsible for scanning the file system for changes. It will produce a _merkle tree_ representing the state of the file system for easy reconciliation with the server. For performance reasons, the scanner will hold an in-memory cache of the last performed scan and will decide to hash or not files and folders depending on the name + size + last modified time of the file or folder. If any of those three attributes have changed, the scanner will hash the file or folder and update the cache, if not will server the last known value.
2. **Watcher**: This component is responsible for monitoring the file system for changes. We use `notify` crate to watch for changes in the file system. The watcher will listen for events, but will not take them at face value, it will just trigger a `scan`.
3. **Syncer**: This component is responsible for synchronizing the local file system with the remote server. It will take the merkle tree produced by the scanner and compare it with the merkle tree received from the server, and will produce a list of changes that need to be made to synchronize the two trees. When first connection or in case of a reconnection, it will start the diffing procedure with the server right away.
4. **Sender**: This component is responsible for sending the changes to the server. It will take the list of changes produced by the syncer and send them to the server in a batch.

Open questions:

- Do we need to persist the merkle tree in the client to a local DB?
- Do we need to persist the scanner cache to disk?

### Phaneros Server

The server is divided into two conceptual planes:

1. The **Control Plane** which is a stateless HTTP API that handles authentication, authorization, and synchronization requests from the client via returning nodes by hash. The control plane addresses tree nodes by their hash and never by their path, so that the client can always query the server for a node by its hash and get the same node, regardless of the path it was queried from, avoiding mid-sync conflicts by making those nodes immutable. A consequence of this is that the server cannot delete a node the moment it becomes unreachable, since an in-flight diff may still ask for it, so unreachable nodes are kept for a grace period before being swept.
2. The **Data Plane** which is directly the storage engine that stores files and their versions, and serves the files to the client. The control plane will be in charge of generting tickets to upload/download those blobs. This service will potentially be S3/R2 or a custom implementation deployed alongside the control plane to give the user the option of self-hosting the server.

The server will feature versioning of files, so that the user can access previous versions of a file if needed. This will be possible thanks to saving the files in a content-addressable storage and the trees in a database, where they could be queried by hash. The server will also feature a garbage collection with mark-and-sweep to remove old versions of files that are no longer needed.

Deleting history is two separate operations. First, version records are pruned according to a retention policy (e.g. keep N versions, or M days). Second, a mark-and-sweep garbage collector walks the trees from all retained version roots, marks every reachable node, and deletes the rest (respecting the grace period above). I chose mark-and-sweep over reference counting because it is idempotent. A crash mid-sweep just means the next run redoes the work, whereas reference counts require atomic updates across many nodes on every commit and every deletion, and a crash mid-decrement leaks or corrupts counts permanently.

#### Control Plane

##### Comunication with the Client

The control plane will expose a REST API that the client will use to communicate with the server. The API will be secured with JWT tokens and will require authentication for all requests.

From the server to the client communication, we will use SSE to notify the client of changes.
