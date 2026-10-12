use muguet_android::runtime::run_runtime_probe;
use serde_json::Value;

fn check<'a>(report: &'a Value, name: &str) -> &'a Value {
    let check = report["checks"]
        .as_array()
        .expect("checks must be an array")
        .iter()
        .find(|check| check["name"] == name)
        .unwrap_or_else(|| panic!("missing {name}: {report}"));
    assert_eq!(check["status"], "passed", "{check}");
    &check["details"]
}

// Wrong guest values, accepted out-of-bound growth, or a missed notification
// must fail these assertions even if the report's overall status says passed.
#[test]
fn observes_guest_atomics_memory_bounds_and_cross_thread_notification() {
    let report = run_runtime_probe();
    assert_eq!(report["status"], "passed", "{report}");
    assert!(report["elapsed_ms"].is_u64(), "{report}");

    let atomics = check(&report, "guest_atomics");
    assert_eq!(atomics["address"], 0);
    assert_eq!(atomics["written"], 305419896);
    assert_eq!(atomics["observed"], 305419896);

    let growth = check(&report, "memory_growth");
    assert_eq!(growth["initial_pages"], 1);
    assert_eq!(growth["grow_previous_pages"], 1);
    assert_eq!(growth["grown_pages"], 2);
    assert_eq!(growth["grown_bytes"], 131072);
    assert_eq!(growth["out_of_bound_growth_rejected"], true);
    assert_eq!(growth["final_pages"], 2);
    assert!(growth["rejection_error"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));

    let threads = check(&report, "thread_wait_notify");
    assert_eq!(threads["wait_result"], 0); // Wasm: woken by notify.
    assert_eq!(threads["notified_waiters"], 1);
    assert_eq!(threads["worker_observed"], 305419896);
    assert_eq!(threads["worker_written"], 591751049);
    assert_eq!(threads["main_observed"], 591751049);
    assert!(threads["notify_attempts"].as_u64().unwrap() >= 1);
}
