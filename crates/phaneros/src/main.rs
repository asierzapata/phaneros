use phaneros::folder_tree::FolderTree;
use phaneros::scanner::Scanner;
use phaneros::watcher::Watcher;

fn main() {
    let watcher = Watcher::new(Scanner::new(
        String::from("/Users/asierzapata/Documents/"),
        false,
    ));

    let (watcher_rx, initial_folder_tree) = watcher.watch().unwrap();

    let mut folder_tree: FolderTree = initial_folder_tree;

    // Listen for folder tree updates
    for updated_folder_tree in watcher_rx {
        println!("Watcher received updated folder tree");
        folder_tree = updated_folder_tree;
    }
}
