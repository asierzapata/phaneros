mod filesystem_watcher;
pub mod remote_listener;

pub use filesystem_watcher::{RescanError, RescanHandle, WatchHandle, Watcher, WatcherError};
pub use remote_listener::{RemoteListenerEvent, spawn_remote_listener};

#[cfg(test)]
mod tests;
