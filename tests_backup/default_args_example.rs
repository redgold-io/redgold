use default_args::default_args;

// this would make a macro named `foo`
// and original function named `foo_`
default_args! {
    fn foo(important_arg: u32, optional: u32 = 100) -> String {
        format!("{}, {}", important_arg, optional)
    }
}

#[test]
fn check_foo() {
    // in other codes ...
    assert_eq!(foo!(1), "1, 100"); // foo(1, 100)
    assert_eq!(foo!(1, 3), "1, 3"); // foo(1, 3)
    assert_eq!(foo!(1, optional = 10), "1, 10"); // foo(1, 10)
}

// let's make another one
default_args! {
    #[inline]
    pub async unsafe extern "C" fn bar<S1, S2, S3>(a: S1, b: S2 = "b", c: S3 = "c") -> String
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
        S3: AsRef<str>,
    {
        format!("{}, {}, {}", a.as_ref(), b.as_ref(), c.as_ref())
    }
    // that was long signature!
}

// this don't work?
//
// #[test]
// fn check_bar() {
// // in other codes ...
//     assert_eq!(unsafe { bar!("a") }.await, "a, b, c");
//     assert_eq!(unsafe { bar!("a", "d") }.await, "a, d, c");
// // you can even mix named & unnamed argument in optional arguments
//     assert_eq!(unsafe { bar!("a", "d", c = "e") }.await, "a, d, e");
//     assert_eq!(unsafe { bar!("a", c = "e") }.await, "a, b, e");
// }
