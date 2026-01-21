pub(crate) enum Union<T, U> {
    First(T),
    Second(U),
}

#[macro_export]
macro_rules! first {
    ($e:expr) => {
        Union::First($e)
    };
}

#[macro_export]
macro_rules! second {
    ($e:expr) => {
        Union::Second($e)
    };
}

#[macro_export]
macro_rules! match_union {
    ($e:expr; $a:ident => $f:expr, $b:ident => $g:expr) => {
        match $e {
            Union::First($a) => $f,
            Union::Second($b) => $g,
        }
    };
}
