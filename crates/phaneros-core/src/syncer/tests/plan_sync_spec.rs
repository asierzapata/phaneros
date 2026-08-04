use crate::node_repository::Hash;
use crate::syncer::{SyncPlan, plan_sync};

fn hash(v: &str) -> Hash {
    v.to_string()
}

#[tokio::test]
async fn no_base_with_a_remote_root_uses_bootstrap_pull_policy() {
    let local = hash("local");

    assert_eq!(
        plan_sync(None, &local, Some(&local)),
        SyncPlan::RemoteBootstrapPull
    );
    assert_eq!(
        plan_sync(None, &local, Some(&hash("remote"))),
        SyncPlan::RemoteBootstrapPull
    );
}

#[tokio::test]
async fn no_base_and_no_remote_root_pushes_local() {
    // First run of the first client on a brand new drive: there is nothing to
    // bootstrap from, so local seeds the remote instead of stalling.
    let local = hash("local");

    assert_eq!(plan_sync(None, &local, None), SyncPlan::LocalPush);
}

#[tokio::test]
async fn local_and_remote_equal_means_converged_even_with_stale_base() {
    let base = hash("old-base");
    let current = hash("current");

    assert_eq!(
        plan_sync(Some(&base), &current, Some(&current)),
        SyncPlan::Converged
    );
}

#[tokio::test]
async fn when_remote_root_is_absent_and_base_exists_we_recover_by_pushing_local() {
    let base = hash("base");

    // Even if local has not changed since base, remote has effectively lost
    // its visible tree (`None` root), so recovering remote from local is the
    // only forward progress strategy.
    assert_eq!(plan_sync(Some(&base), &base, None), SyncPlan::LocalPush);

    let local_new = hash("local-new");
    assert_eq!(
        plan_sync(Some(&base), &local_new, None),
        SyncPlan::LocalPush
    );
}

#[tokio::test]
async fn pull_when_only_remote_changed_since_base() {
    let base = hash("base");
    let remote = hash("remote-new");

    assert_eq!(
        plan_sync(Some(&base), &base, Some(&remote)),
        SyncPlan::RemotePull
    );
}

#[tokio::test]
async fn push_when_only_local_changed_since_base() {
    let base = hash("base");
    let local = hash("local-new");

    assert_eq!(
        plan_sync(Some(&base), &local, Some(&base)),
        SyncPlan::LocalPush
    );
}

#[tokio::test]
async fn merge_when_both_sides_diverged_from_base() {
    let base = hash("base");
    let local = hash("local-new");
    let remote = hash("remote-new");

    assert_eq!(
        plan_sync(Some(&base), &local, Some(&remote)),
        SyncPlan::Merge
    );
}

#[tokio::test]
async fn truth_table_rows_stay_stable() {
    let b = hash("b");
    let l = hash("l");
    let r = hash("r");

    let cases = vec![
        (None, &l, Some(&r), SyncPlan::RemoteBootstrapPull),
        (None, &l, None, SyncPlan::LocalPush),
        (Some(&b), &l, Some(&l), SyncPlan::Converged),
        (Some(&b), &b, Some(&r), SyncPlan::RemotePull),
        (Some(&b), &l, Some(&b), SyncPlan::LocalPush),
        (Some(&b), &l, Some(&r), SyncPlan::Merge),
        (Some(&b), &b, None, SyncPlan::LocalPush),
        (Some(&b), &l, None, SyncPlan::LocalPush),
    ];

    for (base, local, remote, expected) in cases {
        assert_eq!(
            plan_sync(base, local, remote),
            expected,
            "unexpected plan for B={base:?} L={local} R={remote:?}"
        );
    }
}
