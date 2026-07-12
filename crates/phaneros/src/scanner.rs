// Scanner is reponsible for maintaning a local representation of a given path
// and its contents for efficient change detection and reconciliation
// with a remote representation of the same path and its contents.

struct MerkleTree {
    root_hash: String,      // The root hash of the Merkle tree
    nodes: Vec<MerkleNode>, // The nodes of the Merkle tree
}

struct MerkleNode {
    hash: String,              // The hash of the node
    children: Vec<MerkleNode>, // The children of the node
}

struct Scanner {
    file_path: String, // The path to the file or directory being scanned
    tree: MerkleTree,  // The Merkle tree representing the contents of the file or directory
}
