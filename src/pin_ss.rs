use std::{marker::PhantomPinned, pin::Pin};

struct SS {
    s: String,
    _a: PhantomPinned,
}

fn main() {
    let mut s = SS {
        _a: PhantomPinned,
        s: String::from("123"),
    };
    let  p = unsafe { Pin::new_unchecked(&mut s) };
    let j = unsafe { p.get_unchecked_mut() };
    dbg!(&j.s);
}