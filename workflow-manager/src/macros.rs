macro_rules! ensure {
    ($cond:expr, $($err:tt)*) => {
        if !$cond {
            return Err(anyhow::anyhow!($($err)*).into());
        }
    };
}

pub(crate) use ensure;