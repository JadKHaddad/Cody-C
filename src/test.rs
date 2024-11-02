pub fn init_tracing() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish(),
    )
    .ok();
}

#[cfg(feature = "bincode")]
pub mod bincode {
    use bincode::serde::Compat as BincodeSerdeCompat;

    #[derive(bincode::Encode, bincode::Decode)]
    pub enum BincodeMessage {
        Numbers(u32, u32, u32),
        String(BincodeSerdeCompat<heapless::String<32>>),
        Vec(BincodeSerdeCompat<heapless::Vec<u8, 32>>),
    }

    impl core::fmt::Debug for BincodeMessage {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::Numbers(a, b, c) => write!(f, "Numbers({}, {}, {})", a, b, c),
                Self::String(s) => write!(f, "String({})", s.0),
                Self::Vec(v) => write!(f, "Vec({:?})", v.0),
            }
        }
    }

    impl Clone for BincodeMessage {
        fn clone(&self) -> Self {
            match self {
                Self::Numbers(a, b, c) => Self::Numbers(*a, *b, *c),
                Self::String(s) => Self::String(BincodeSerdeCompat(s.0.clone())),
                Self::Vec(v) => Self::Vec(BincodeSerdeCompat(v.0.clone())),
            }
        }
    }

    impl PartialEq for BincodeMessage {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::Numbers(a, b, c), Self::Numbers(x, y, z)) => a == x && b == y && c == z,
                (Self::String(s), Self::String(t)) => s.0 == t.0,
                (Self::Vec(v), Self::Vec(w)) => v.0 == w.0,
                _ => false,
            }
        }
    }
}
