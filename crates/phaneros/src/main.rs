use phaneros::syncer::Syncer;
use phaneros::watcher::Watcher;

fn main() {
    let watcher = Watcher::new(String::from(
        "/Users/asierzapata/Documents/Projects/phaneros/documentation",
    ));

    println!("Watcher started, waiting for changes...");

    // TODO: Handle the error properly instead of unwrapping.
    let (watcher_rx, initial_root_hash, node_store) = watcher.watch().unwrap();

    let syncer = Syncer::new(watcher_rx, initial_root_hash, node_store);

    syncer.run();
}
