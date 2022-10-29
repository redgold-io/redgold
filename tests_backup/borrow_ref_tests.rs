use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

struct A {
    a: u32,
}

struct B {
    a: A,
}

struct C {
    a: A,
}

// http://gradebot.org/doc/ipur/concurrency.html
#[test]
fn debug_borrow() {
    let a = A { a: 10 as u32 };

    let b = B { a: a };

    // borrow issue -- requires clone to access value.
    //let c = C { a: a };
}

struct A2 {
    a: u32,
}

impl A2 {
    async fn run(&mut self) {
        loop {
            sleep(Duration::from_secs(1));
            self.a += 1;
        }
    }
}

struct B2 {
    a: Arc<A2>,
}

struct C2 {
    a: Arc<A2>,
}

// http://gradebot.org/doc/ipur/concurrency.html
#[tokio::test]
async fn debug_borrow2() {
    let a = A2 { a: 10 as u32 };
    let arc = Arc::new(a);

    let b = B2 { a: arc.clone() };

    // borrow issue
    let c = C2 { a: arc.clone() };

    println!("{:?}", b.a.a);
    println!("{:?}", c.a.a);
    // {
    //     let mut arc1 = arc.clone();
    //     let mut t = arc1.borrow_mut();
    //     t.a += 1;
    // }
    // println!("{:?}", b.a.a);
    // println!("{:?}", c.a.a);

    //  println!("{:?}", a.a);
}
//
// struct B3 {
//     a: Arc<Mutex<A>>,
// }
//
// impl B3 {
//     fn update(&self) {
//         let m = self.a.lock().unwrap();
//         m.a += 1;
//     }
// }
//
// struct C3 {
//     a: Arc<Mutex<A>>,
// }
//
// // http://gradebot.org/doc/ipur/concurrency.html
// #[test]
// fn debug_borrow3() {
//     let a = A { a: 10 as u32 };
//     let mutex = Mutex::new(a);
//     // let arc2 = Arc::new(a);
//     let arc = Arc::new(mutex);
//
//     let b = B3 { a: arc };
//
//     // borrow issue
//     let mut c = C3 { a: arc.clone() };
//
//     b.update();
//     println!("{:?}", c.a.get_mut().unwrap().a);
// }
// // https://www.reddit.com/r/rust/comments/8j6syr/rwlock_that_doesnt_block_either_side_allows_for/
// // http://web.mit.edu/rust-lang_v1.25/arch/amd64_ubuntu1404/share/doc/rust/html/std/sync/struct.RwLock.html
// //https://stackoverflow.com/questions/62851910/write-while-multiple-readers-are-reading-from-different-threads-in-rust
