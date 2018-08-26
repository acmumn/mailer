/// A Context "literal".
macro_rules! context {
    ($($name:ident : $value:expr),* $(,)*) => {{
        let mut ctx = ::tera::Context::new();
        $(ctx.add(stringify!($name), &$value);)*
        ctx
    }};
}
