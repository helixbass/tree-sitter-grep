#[macro_export]
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

#[macro_export]
macro_rules! return_if_none {
    ($expr:expr $(,)?) => {
        match $expr {
            None => {
                return;
            }
            Some(expr) => expr,
        }
    };
}
