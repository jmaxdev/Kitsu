use tempfile::tempdir;

#[test]
fn test_cli_persona_remove_and_issue_commands() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    let repo = kitsu_core::Repository::init(repo_path).unwrap();
    assert!(repo.repo_dir().exists());

    let mut store = kitsu_core::identity::IdentityStore::load(repo_path);
    let mut p = kitsu_core::identity::Identity {
        id: "feature-bot".into(),
        name: "Feature Bot".into(),
        email: "bot@kitsu.dev".into(),
        public_key: None,
        private_key: None,
    };
    p.generate_keys();
    store.identities.push(p);
    store.save(repo_path, false).unwrap();

    assert!(store.identities.iter().any(|i| i.id == "feature-bot"));
    assert!(store.remove("feature-bot").unwrap());
    store.save(repo_path, false).unwrap();
    assert!(!store.identities.iter().any(|i| i.id == "feature-bot"));

    let repo_dir = repo.repo_dir();
    let issue = kitsu_core::issues::LocalIssueManager::create(
        &repo_dir,
        "CLI Integration Bug",
        "Details of issue",
        "Test User",
    )
    .unwrap();
    assert_eq!(issue.id, 1);
    assert_eq!(issue.state, "open");

    let closed = kitsu_core::issues::LocalIssueManager::close(
        &repo_dir,
        1,
        Some("Resolved in integration test"),
    )
    .unwrap();
    assert_eq!(closed.state, "closed");
    assert_eq!(
        closed.close_comment.as_deref(),
        Some("Resolved in integration test")
    );

    let issue_file = repo_dir.join("issues/1.toml");
    assert!(issue_file.exists());

    let deleted = kitsu_core::issues::LocalIssueManager::delete(&repo_dir, 1).unwrap();
    assert!(deleted);
    assert!(!issue_file.exists());
}
