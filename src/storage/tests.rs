use crate::storage::DenseStorage;

#[test]
fn invalid_id_test() {
    let s = DenseStorage::<GridCell>::new_rect(10, 0, 0, 1, 1);

    assert_eq!(s.cell_id(Point2 { x: 15.0, y: 15.0 }), 0);
    assert_eq!(s.cell_id(Point2 { x: 5.0, y: 15.0 }), 0);
    assert_eq!(s.cell_id(Point2 { x: 15.0, y: 5.0 }), 0);
    assert_eq!(s.cell_id(Point2 { x: 5.0, y: 5.0 }), 0);
    assert_eq!(s.cell_id(Point2 { x: -15.0, y: 15.0 }), 0);
    assert_eq!(s.cell_id(Point2 { x: 5.0, y: -15.0 }), 0);
    assert_eq!(s.cell_id(Point2 { x: 15.0, y: -5.0 }), 0);
    assert_eq!(s.cell_id(Point2 { x: -5.0, y: 5.0 }), 0);
}

#[test]
fn test_dense_iter_manual() {
    let x = DenseIter {
        ur: 8,
        width: 5,
        diff: 3,
        c: 0,
        cur: 1,
    };

    assert_eq!(x.collect::<Vec<_>>(), vec![1, 2, 3, 6, 7, 8])
}

#[test]
fn test_dense_iter() {
    let s = DenseStorage::<GridCell>::new_rect(10, 0, 0, 5, 2);

    assert_eq!(
        s.cell_range(1, 8).collect::<Vec<_>>(),
        vec![1, 2, 3, 6, 7, 8]
    )
}
