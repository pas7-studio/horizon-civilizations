    use super::super::*;

    #[test]
    fn boundaries_follow_host_clock_rollovers_and_rewinds() {
        use DailyPhase::{Dawn, Evening, Night, Work};
        for (tick, expected) in [
            (0, Dawn), (1_999, Dawn), (2_000, Work),
            (10_999, Work), (11_000, Evening), (12_999, Evening),
            (13_000, Night), (23_999, Night), (24_000, Dawn),
            (-1, Night), (-24_000, Dawn), (26_000, Work),
        ] {
            assert_eq!(DailyPhase::at(tick), expected, "clock tick {tick}");
        }
        assert_eq!(
            DailyPhase::at(i64::MAX),
            DailyPhase::at(i64::MAX.rem_euclid(24_000))
        );
        assert_eq!(
            DailyPhase::at(i64::MIN),
            DailyPhase::at(i64::MIN.rem_euclid(24_000))
        );
    }

    #[test]
    fn guard_watch_continues_while_other_jobs_stop_at_night() {
        for phase in [
            DailyPhase::Dawn, DailyPhase::Work,
            DailyPhase::Evening, DailyPhase::Night,
        ] {
            assert!(phase.on_shift(Job::Guard));
        }
        for job in [
            Job::Farmer, Job::Baker, Job::Lumberjack,
            Job::Miner, Job::Builder, Job::Merchant,
        ] {
            assert!(DailyPhase::Work.on_shift(job));
            assert!(!DailyPhase::Dawn.on_shift(job));
            assert!(!DailyPhase::Evening.on_shift(job));
            assert!(!DailyPhase::Night.on_shift(job));
        }
    }

    #[test]
    fn off_duty_routes_are_stable_bounded_and_require_no_save_change() {
        for id in 0..200 {
            for phase in [
                DailyPhase::Dawn, DailyPhase::Evening,
                DailyPhase::Night,
            ] {
                let offset = phase.offset(Job::Farmer, id);
                assert_eq!(offset, phase.offset(Job::Farmer, id));
                assert!((-4..=4).contains(&offset[0]));
                assert_eq!(offset[1], 0);
                assert!((-5..=4).contains(&offset[2]));
            }
            assert_eq!(
                DailyPhase::Night.offset(Job::Guard, id),
                Job::Guard.workplace(id)
            );
            assert_eq!(
                DailyPhase::Work.offset(Job::Builder, id),
                Job::Builder.workplace(id)
            );
        }
        let encoded = serde_json::to_value(World::default()).unwrap();
        assert!(encoded.get("daily_phase").is_none());
        assert!(encoded.get("routine").is_none());
        let recovered: World = serde_json::from_value(encoded).unwrap();
        recovered.validate().unwrap();
    }
