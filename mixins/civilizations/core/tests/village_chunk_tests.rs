    use super::super::*;

    #[test]
    fn only_the_civic_block_is_held_for_a_village_with_no_workplaces() {
        let footprint = village_chunk_footprint([0, 70, 0], &[]);
        assert_eq!(footprint.len(), 9);
        assert!(footprint.contains(&[0, 0]));
        assert!(footprint.contains(&[-1, -1]) && footprint.contains(&[1, 1]));
        assert!(!footprint.contains(&[2, 0]));
    }

    #[test]
    fn workplaces_and_their_walking_corridor_are_held_not_a_square() {
        let center = [0, 70, 0];
        let camp = [64, 70, 0];
        let footprint = village_chunk_footprint(center, &[camp]);
        assert!(footprint.contains(&[4, 0]));
        // Every chunk the straight path crosses is present.
        for chunk in 0..=4 {
            assert!(footprint.contains(&[chunk, 0]), "missing corridor chunk {chunk}");
        }
        // The square that the old fixed radius would have held is not used.
        assert!(!footprint.contains(&[0, 3]));
        assert!(!footprint.contains(&[3, 3]));
        assert!(footprint.len() < 25);
    }

    #[test]
    fn a_far_workplace_never_blows_the_budget() {
        let center = [0, 70, 0];
        let far: Vec<[i32; 3]> = (0..4).map(|i| [4_000 + i * 100, 70, 4_000]).collect();
        let footprint = village_chunk_footprint(center, &far);
        assert!(footprint.len() <= MAX_HELD_VILLAGE_CHUNKS);
        assert!(footprint.contains(&[0, 0]));
        assert_eq!(
            village_chunk_footprint(center, &[[64, 70, 0]]),
            village_chunk_footprint(center, &[[64, 70, 0]])
        );
    }

    #[test]
    fn negative_coordinates_stay_in_the_right_chunk() {
        let footprint = village_chunk_footprint([-99, 69, -327], &[[-116, 68, -363]]);
        assert!(footprint.contains(&[-7, -21]));
        assert!(footprint.contains(&[-8, -23]));
        assert!(footprint.iter().all(|chunk| chunk[1] <= -19));
    }
