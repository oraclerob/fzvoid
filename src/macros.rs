#[macro_export]
macro_rules! p{
    ($($elem: expr),+ ) => {
        println!(concat!($(concat!(stringify!($elem), " - {:?}\n")),+), $($elem),+);
    };
}
