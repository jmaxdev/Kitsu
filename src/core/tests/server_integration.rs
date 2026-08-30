extern crate core as kitsu_core;

use kitsu_core::server::{ServerToken, run_server};
use std::thread;
use std::time::Duration;

#[test]
fn test_local_server_auth_and_endpoints() {
    let test_port = 5919;

    thread::spawn(move || {
        let _ = run_server(test_port);
    });

    thread::sleep(Duration::from_millis(200));

    let health_resp = ureq::get(&format!("http://127.0.0.1:{}/api/v1/health", test_port))
        .call()
        .unwrap();
    assert_eq!(health_resp.status(), 200);

    let unauth_resp = ureq::get(&format!("http://127.0.0.1:{}/api/v1/status", test_port)).call();
    assert!(unauth_resp.is_err());
    if let Err(ureq::Error::Status(code, _)) = unauth_resp {
        assert_eq!(code, 401);
    }

    let token = ServerToken::get_or_create().unwrap();
    let status_resp = ureq::get(&format!("http://127.0.0.1:{}/api/v1/status", test_port))
        .set("Authorization", &format!("Bearer {}", token))
        .call()
        .unwrap();
    assert_eq!(status_resp.status(), 200);
    let body: serde_json::Value = status_resp.into_json().unwrap();
    assert_eq!(body["status"], "running");

    let repos_resp = ureq::get(&format!(
        "http://127.0.0.1:{}/api/v1/repositories",
        test_port
    ))
    .set("Authorization", &format!("Bearer {}", token))
    .call()
    .unwrap();
    assert_eq!(repos_resp.status(), 200);

    let shutdown_resp = ureq::post(&format!("http://127.0.0.1:{}/api/v1/shutdown", test_port))
        .set("Authorization", &format!("Bearer {}", token))
        .call()
        .unwrap();
    assert_eq!(shutdown_resp.status(), 200);
}
