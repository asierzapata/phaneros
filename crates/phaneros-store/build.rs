fn main() {
    // Recompile when migrations change so `sqlx::migrate!()` re-embeds them.
    println!("cargo:rerun-if-changed=migrations");
}
