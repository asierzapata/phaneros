/// A merkle tree representation of a file or directory, where each node is a hash of its contents and its children.
#[derive(Debug)]
pub struct FolderTree {
    pub root_hash: String,          // The root hash of the tree
    pub nodes: Vec<FolderTreeNode>, // The nodes of the tree
}

/// A node in the merkle tree, representing a file or directory and its hash.
#[derive(Debug, Clone)]
pub struct FolderTreeNode {
    pub name: String, // The path of the file or directory represented by the node. Just for debugging, delete later
    pub hash: String, // The hash of the node
    pub children: Vec<FolderTreeNode>, // The children of the node
}
