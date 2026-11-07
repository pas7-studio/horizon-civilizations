// Horizon adapter unit test: compiled only by the native cargo test gate.
use super::*;

    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    fn sandbox() -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!(
            "horizon-civ-save-{}-{nanos}-{}",
            std::process::id(), NEXT_DIR.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn interrupted_first_write_cannot_look_like_a_new_empty_world() {
        let path = sandbox();
        let pending = path.join("world.json.tmp");
        fs::write(&pending, b"{ interrupted first write").unwrap();
        let loaded = load(path.clone());
        assert!(loaded.world.is_none());
        assert!(loaded.problem.as_deref().unwrap().contains("world.json.tmp"));
        assert_eq!(fs::read(&pending).unwrap(), b"{ interrupted first write");
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn a_valid_primary_does_not_silently_discard_an_abandoned_tmp() {
        let path = sandbox();
        let world = World::default();
        fs::write(path.join(SAVE), serde_json::to_vec(&world).unwrap()).unwrap();
        fs::write(path.join("world.json.tmp"), b"unfinished newer state").unwrap();
        let loaded = load(path.clone());
        assert!(loaded.world.is_none());
        assert!(loaded.problem.as_deref().unwrap().contains("world.json.tmp"));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn stale_tmp_is_never_truncated_by_a_later_save() {
        let path = sandbox();
        let pending = path.join("world.json.tmp");
        fs::write(&pending, b"recoverable pending bytes").unwrap();
        let state = ActiveWorld {
            path: path.clone(),
            world: Some(World::default()),
            problem: None,
            physical_ticks: 0,
        };
        let failure = save(&state).unwrap_err();
        assert!(failure.contains("world.json.tmp"));
        assert_eq!(fs::read(&pending).unwrap(), b"recoverable pending bytes");
        assert!(!path.join(SAVE).exists());
        fs::remove_dir_all(path).unwrap();
    }
