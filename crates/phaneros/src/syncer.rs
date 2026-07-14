use crate::folder_tree::IndexTree;

pub struct Syncer {
    watcher_rx: std::sync::mpsc::Receiver<IndexTree>,
    initial_folder_tree: IndexTree,
}

impl Syncer {
    pub fn new(
        watcher_rx: std::sync::mpsc::Receiver<IndexTree>,
        initial_folder_tree: IndexTree,
    ) -> Self {
        Syncer {
            watcher_rx,
            initial_folder_tree,
        }
    }

    pub fn run(&self) {
        println!(
            "Syncer started with initial folder tree: {:?}",
            self.initial_folder_tree
        );
        for updated_folder_tree in &self.watcher_rx {
            println!("Syncer received updated folder tree");
            println!("Updated folder tree: {:?}", updated_folder_tree);
            // Here you can implement the logic to sync the updated folder tree with your database or any other storage.
        }
    }
}
