use ndarray::arr2;

#[test]
fn debug() {
    let a = arr2(&[[1., 0.9, 0.5], [0.8, 1., 0.2], [0.3, 0.4, 1.]]);

    println!("{:?}", a);
}

// https://docs.rs/dot/0.1.4/dot/
// https://crates.io/crates/tabbycat
// https://github.com/datproject/dat
// https://www.reddit.com/r/rust/comments/g3ub83/is_anyone_using_rust_analyzer_editing_on_remote/
