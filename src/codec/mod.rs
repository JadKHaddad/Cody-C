//! A ready to use set of codecs.

pub mod any;
pub mod bytes;
pub mod lines;

pub use self::{any::*, bytes::*, lines::*};
