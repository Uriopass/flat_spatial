fn main() {
    use flat_spatial::Grid;

    let mut g: Grid<()> = Grid::new(10);
    let a = g.insert([3.0, 3.0], ());
    let _b = g.insert([12.0, -8.0], ());

    let around: Vec<_> = g
        .query_around([2.0, 2.0], 5.0)
        .map(|(id, _pos)| id)
        .collect();

    println!("{:?} = {:?}", vec![a], around);
}
