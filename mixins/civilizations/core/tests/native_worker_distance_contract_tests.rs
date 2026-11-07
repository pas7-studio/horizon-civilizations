    use super::super::*;

    #[test]
    fn worker_directly_at_hall_is_near_but_same_xz_deep_below_is_not() {
        let hall = [20, 70, 35];
        assert!(native_worker_near([20.5, 70.0, 35.5], hall, 8.0, 4.0));
        assert!(!native_worker_near([20.5, 45.0, 35.5], hall, 8.0, 4.0));
        assert!(!native_worker_near([20.5, 93.0, 35.5], hall, 8.0, 4.0));
        assert!(!native_worker_near([40.5, 70.0, 35.5], hall, 8.0, 4.0));
    }

    #[test]
    fn worker_can_reach_verified_short_tree_without_felling_distant_trunks() {
        let feet = [17.5, 70.0, 0.5];
        assert!(native_worker_reaches_block(feet, [16, 70, 0], 4.5));
        assert!(native_worker_reaches_block(feet, [16, 73, 0], 4.5));
        assert!(!native_worker_reaches_block(feet, [16, 76, 0], 4.5));
        assert!(!native_worker_reaches_block(feet, [30, 73, 0], 4.5));
        assert!(!native_worker_reaches_block([17.5, 40.0, 0.5], [16, 73, 0], 4.5));
    }

    #[test]
    fn invalid_actor_or_radius_cannot_authorize_native_delivery() {
        assert!(!native_worker_near(
            [f64::NAN, 70.0, 0.5], [0, 70, 0], 8.0, 4.0
        ));
        assert!(!native_worker_near(
            [0.5, 70.0, 0.5], [0, 70, 0], f64::INFINITY, 4.0
        ));
        assert!(!native_worker_near(
            [0.5, 70.0, 0.5], [0, 70, 0], 8.0, -1.0
        ));
        assert!(!native_worker_reaches_block(
            [0.5, f64::INFINITY, 0.5], [0, 70, 0], 4.5
        ));
    }
