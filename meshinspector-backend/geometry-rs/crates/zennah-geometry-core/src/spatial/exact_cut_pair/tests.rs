use super::source_preserving::*;

#[test]
fn source_preserving_meshlib_like_replacement_sources_expand_removed_face_hits() {
    let expanded =
        source_preserving_meshlib_like_replacement_source_faces(&[vec![7, 7, 4, 5, 5, 4]]);

    assert_eq!(
        expanded,
        vec![vec![
            7, 7, 7, 7, 7, // two contour hits in a removed triangle become five records
            4, 4, 4, 4, // the primary separated run carries the replacement records
            5, 5, 5, 5, 5,
            4, // later separated runs keep their direct contour records in order
        ]]
    );
    assert_eq!(
        source_face_counts_by_path(&expanded),
        vec![vec![[4, 5], [5, 5], [7, 5]]]
    );
    assert_eq!(
        source_face_runs_by_path(&expanded),
        vec![vec![[7, 5], [4, 4], [5, 5], [4, 1]]]
    );
    assert_eq!(
        source_preserving_meshlib_like_cut2origin_source_faces(3, &expanded),
        vec![vec![0, 1, 2, 7, 7, 7, 7, 7, 4, 4, 4, 4, 5, 5, 5, 5, 5, 4,]]
    );
}

#[test]
fn source_preserving_meshlib_like_replacement_lifecycle_runs_count_collapsed_hits() {
    let runs = source_preserving_meshlib_like_replacement_lifecycle_runs(
        &[vec![7, 7, 4, 5, 5, 4]],
        &[vec![false, false, true, false, true, false]],
    );

    assert_eq!(
        runs,
        vec![vec![[7, 2, 0, 5], [4, 1, 1, 4], [5, 2, 1, 5], [4, 1, 0, 1],]]
    );
}

#[test]
fn source_preserving_meshlib_like_replacement_lifecycle_slot_runs_track_append_ranges() {
    let slot_runs = source_preserving_meshlib_like_replacement_lifecycle_slot_runs(
        3,
        &[vec![[7, 2, 0, 5], [4, 1, 1, 4], [5, 2, 1, 5], [4, 1, 0, 1]]],
    );

    assert_eq!(
        slot_runs,
        vec![vec![
            [0, 0, 7, 2, 0, 5, 3, 8],
            [0, 1, 4, 1, 1, 4, 8, 12],
            [0, 2, 5, 2, 1, 5, 12, 17],
            [0, 3, 4, 1, 0, 1, 17, 18],
        ]]
    );
}

#[test]
fn source_preserving_meshlib_like_replacement_sources_fold_circular_runs() {
    let expanded =
        source_preserving_meshlib_like_replacement_source_faces(&[vec![3, 2, 2, 3, 9, 3]]);

    assert_eq!(
        expanded,
        vec![vec![
            3, 3, 3, 3, 3, 3, // first and last circular runs are one removed face section
            2, 2, 2, 2, 2, 3, // middle separated run stays in contour order
            9, 9, 9,
        ]]
    );
    assert_eq!(
        source_face_counts_by_path(&expanded),
        vec![vec![[2, 5], [3, 7], [9, 3]]]
    );
    assert_eq!(
        source_face_runs_by_path(&expanded),
        vec![vec![[3, 6], [2, 5], [3, 1], [9, 3]]]
    );
}
