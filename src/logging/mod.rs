//! Logging untilities.

#![allow(missing_docs)]

pub mod formatter;

#[macro_export]
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

#[macro_export]
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

#[macro_export]
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

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::info!($($arg)*);

        #[cfg(feature = "log")]
        log::info!($($arg)*);

        #[cfg(feature = "defmt")]
        defmt::info!($($arg)*);
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::warn!($($arg)*);

        #[cfg(feature = "log")]
        log::warn!($($arg)*);

        #[cfg(feature = "defmt")]
        defmt::warn!($($arg)*);
    };
}
