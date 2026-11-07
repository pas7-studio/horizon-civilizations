    use super::super::*;

    fn world() -> World {
        let mut w = World::default();
        w.found("Ashford", "minecraft:overworld", [0, 70, 0], "operator").unwrap();
        w
    }

    #[test]
    fn bootstrap_is_nonproductive_and_legacy_json_is_compatible() {
        let mut w = world();
        let before = w.settlements[&1].stock.clone();
        assert!(w.observe_host_day(5_000).unwrap());
        assert_eq!(w.day, 0);
        assert_eq!(w.settlements[&1].stock, before);
        assert_eq!(w.last_host_day, Some(5_000));
        let value = serde_json::to_value(&w).unwrap();
        let mut old = value;
        old.as_object_mut().unwrap().remove("last_host_day");
        let mut loaded: World = serde_json::from_value(old).unwrap();
        assert_eq!(loaded.last_host_day, None);
        assert!(loaded.observe_host_day(5_100).unwrap());
        assert_eq!(loaded.day, 0);
        loaded.validate().unwrap();
    }

    #[test]
    fn restart_and_clock_rewinds_never_double_apply_day() {
        let mut w = world();
        w.observe_host_day(100).unwrap();
        assert!(w.observe_host_day(101).unwrap());
        assert_eq!(w.day, 1);
        assert!(!w.observe_host_day(101).unwrap());
        assert!(!w.observe_host_day(90).unwrap());
        assert_eq!(w.day, 1);
        let mut reloaded: World = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert!(!reloaded.observe_host_day(100).unwrap());
        assert!(!reloaded.observe_host_day(101).unwrap());
        assert_eq!(reloaded.day, 1);
        assert!(reloaded.observe_host_day(102).unwrap());
        assert_eq!(reloaded.day, 2);
        reloaded.validate().unwrap();
    }

    #[test]
    fn fast_forward_is_bounded_and_does_not_repeat_on_followup_tick() {
        let mut w = world();
        w.observe_host_day(5).unwrap();
        assert!(w.observe_host_day(5 + 100_000).unwrap());
        assert_eq!(w.day, u64::from(MAX_HOST_CATCHUP_DAYS));
        assert_eq!(w.last_host_day, Some(100_005));
        assert!(!w.observe_host_day(100_005).unwrap());
        assert_eq!(w.day, u64::from(MAX_HOST_CATCHUP_DAYS));
        assert!(w.observe_host_day(100_006).unwrap());
        assert_eq!(w.day, u64::from(MAX_HOST_CATCHUP_DAYS) + 1);
    }

    #[test]
    fn extremely_large_time_delta_is_safe_from_arithmetic_overflow() {
        let mut w = world();
        w.observe_host_day(i64::MIN).unwrap();
        w.observe_host_day(i64::MAX).unwrap();
        assert_eq!(w.day, u64::from(MAX_HOST_CATCHUP_DAYS));
        w.validate().unwrap();
    }
