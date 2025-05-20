#![allow(missing_docs)]

pub fn init_tracing() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish(),
    )
    .ok();
}

#[macro_export]
macro_rules! framed_read {
    ($items:ident, $expected:ident, $decoder:ident) => {
        framed_read!($items, $expected, $decoder, 1024, 1024);
    };
    ($items:ident, $expected:ident, $decoder:ident, $buffer_size:literal) => {
        framed_read!($items, $expected, $decoder, $buffer_size, 1024);
    };
    ($items:ident, $expected:ident, $decoder:ident, $buffer_size:literal $(, $err:ident )?) => {
        framed_read!($items, $expected, $decoder, $buffer_size, 1024 $(, $err )?);
    };
    ($items:ident, $expected:ident, $decoder:ident, $buffer_size:literal, $duplex_max_size:literal $(, $err:ident )?) => {
        let decoder_clone = $decoder.clone();
        let mut collected = Vec::<Vec<u8>>::new();

        let (read, mut write) = tokio::io::duplex($duplex_max_size);

        tokio::spawn(async move {
            for item in $items {
                write.write_all(item.as_ref()).await.expect("Must write");
            }
        });

        let mut framer =
            FramedRead::new_with_buffer(decoder_clone, Compat::new(read), [0_u8; $buffer_size]);

        loop {
            match framer.read_frame().await {
                Ok(Some(item)) => {
                    collected.push(item.into());
                }
                Ok(None) => {}
                Err(_err) => {
                    error!("Error: {:?}", _err);

                    $(
                        assert!(matches!(_err, FramedReadError::$err));
                    )?

                    break;
                }
            }
        }

        assert_eq!($expected, collected);
    };
}

#[macro_export]
macro_rules! sink_stream {
    ($encoder:ident, $decoder:ident, $items:ident) => {
        let items_clone = $items.clone();

        let (read, write) = tokio::io::duplex(1024);

        tokio::spawn(async move {
            let mut writer =
                FramedWrite::new_with_buffer($encoder, Compat::new(write), [0_u8; 1024]);
            let sink = writer.sink();

            pin_mut!(sink);

            for item in items_clone {
                sink.send(item).await.expect("Must send");
            }
        });

        let mut framer = FramedRead::new_with_buffer($decoder, Compat::new(read), [0_u8; 1024]);

        let stream = framer.stream();

        let collected = stream
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        assert_eq!($items, collected);
    };
}
