//! Logging untilities.

pub mod formatter;

macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::trace!($($arg)*);
    };
}
