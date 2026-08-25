extern crate core as kitsu_core;

use kitsu_core::Repository;
use kitsu_core::identity::IdentityStore;
use kitsu_core::objects::{Checkpoint, Chunk};
use kitsu_core::storage::Stage;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_repository_full_lifecycle() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    let repo = Repository::init(repo_path).unwrap();
    assert!(repo.repo_dir().exists());
    assert_eq!(repo.head_hash().unwrap(), None);

    let file1_path = repo_path.join("hello.txt");
    fs::write(&file1_path, b"Hello Kitsu VCS\n").unwrap();

    let mut stage = Stage::load(repo_path, repo.config().clone()).unwrap();
    let chunk_hash = Chunk::new(fs::read(&file1_path).unwrap())
        .save(repo.storage())
        .unwrap();
    stage.add("hello.txt".into(), chunk_hash, 0o100644, 16);
    stage.save().unwrap();

    let map_hash = stage.write_map(repo.storage()).unwrap();
    let id_store = IdentityStore::load(repo_path);
    let active = id_store.get_active();

    let cp = Checkpoint {
        map_hash,
        parent_hash: None,
        author: format!("{} <{}>", active.name, active.email),
        message: "feat: initial commit".into(),
        timestamp: 1700000000,
        signature: None,
    };
    let cp_hash = cp.save(repo.storage()).unwrap();
    repo.update_head(&cp_hash).unwrap();

    assert_eq!(repo.head_hash().unwrap(), Some(cp_hash.clone()));

    fs::write(&file1_path, b"Modified content\n").unwrap();
    let state =
        kitsu_core::state::compute_state(repo_path, repo.config(), repo.storage(), repo.exclude())
            .unwrap();
    assert!(!state.is_clean());
    assert_eq!(state.unstaged_modified, vec!["hello.txt".to_string()]);

    repo.apply_map_to_disk(&cp.map_hash, repo_path).unwrap();
    assert_eq!(
        fs::read(&file1_path).unwrap(),
        b"Hello Kitsu VCS\n".to_vec()
    );
}
