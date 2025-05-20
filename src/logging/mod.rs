//! Logging utilities.

mod formatter;

#[cfg(any(feature = "log", feature = "defmt", feature = "tracing"))]
pub(crate) use formatter::Formatter;

macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::trace!($($arg)*);

        #[cfg(feature = "log")]
        log::trace!($($arg)*);

        #[cfg(feature = "defmt")]
        defmt::trace!($($arg)*);
    };
}

macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::debug!($($arg)*);

        #[cfg(feature = "log")]
        log::debug!($($arg)*);

        #[cfg(feature = "defmt")]
        defmt::debug!($($arg)*);
    };
}

macro_rules! error {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::error!($($arg)*);

        #[cfg(feature = "log")]
        log::error!($($arg)*);

        #[cfg(feature = "defmt")]
        defmt::error!($($arg)*);
    };
}

macro_rules! warn_ {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::warn!($($arg)*);

        #[cfg(feature = "log")]
        log::warn!($($arg)*);

        #[cfg(feature = "defmt")]
        defmt::warn!($($arg)*);
    };
}

pub(crate) use debug;
pub(crate) use error;
pub(crate) use trace;
pub(crate) use warn_ as warn;
