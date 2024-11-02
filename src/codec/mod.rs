//! A ready to use set of codecs.

pub mod any;
pub mod bytes;
pub mod length;
pub mod lines;

pub use self::{any::*, bytes::*, length::*, lines::*};

#[cfg(feature = "bincode")]
#[cfg_attr(docsrs, doc(cfg(feature = "bincode")))]
pub mod bincode;

#[cfg(feature = "bincode")]
pub use self::bincode::*;
