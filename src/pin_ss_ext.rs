use pin_project_lite::pin_project;
use std::pin::Pin;
use std::marker::PhantomPinned;

pin_project! {
    #[derive(Debug)]
    struct SS {
        #[pin]
        s: String,
        _a: PhantomPinned,
    }
}

fn main() {
    let mut s = SS {
        _a: PhantomPinned,
        s: String::from("123"),
    };
    let p = unsafe { Pin::new_unchecked(&mut s) };

    let mut this = p.project();
    this.s.push_str("456");
    println!("s: {}", this.s);
    println!("{:?}", s);
}
