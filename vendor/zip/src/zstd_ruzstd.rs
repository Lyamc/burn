//! Zstandard zip support backed by [`ruzstd`](https://github.com/KillingSpark/zstd-rs).
//!
//! The compressor buffers the member and encodes it when the writer finishes.
//! ruzstd currently implements its fastest level only, so every accepted zip
//! level is encoded at that setting. Frames still decode with ordinary zstd.

use std::io::{self, Read};

/// Same default the C `zstd` crate advertises to zip (level 3).
pub(crate) const DEFAULT_COMPRESSION_LEVEL: i32 = 3;

/// Level window zip already documents for Zstandard members.
pub(crate) fn compression_level_range() -> std::ops::RangeInclusive<i32> {
    -7..=22
}

pub(crate) struct Decoder<R: Read> {
    inner: ruzstd::decoding::StreamingDecoder<R, ruzstd::decoding::FrameDecoder>,
}

impl<R: Read> Decoder<R> {
    pub(crate) fn with_buffer(reader: R) -> io::Result<Self> {
        let inner = ruzstd::decoding::StreamingDecoder::new(reader).map_err(|err| {
            io::Error::new(io::ErrorKind::InvalidData, format!("zstd decode: {err:?}"))
        })?;
        Ok(Self { inner })
    }

    pub(crate) fn finish(self) -> R {
        self.inner.into_inner()
    }
}

impl<R: Read> Read for Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

pub(crate) mod stream {
    pub(crate) mod write {
        use std::io::{self, Write};

        use ruzstd::encoding::{compress, CompressionLevel};

        pub(crate) struct Encoder<W: Write> {
            inner: W,
            buf: Vec<u8>,
            level: i32,
        }

        impl<W: Write> Encoder<W> {
            pub(crate) fn new(inner: W, level: i32) -> io::Result<Self> {
                Ok(Self {
                    inner,
                    buf: Vec::new(),
                    level,
                })
            }

            pub(crate) fn get_ref(&self) -> &W {
                &self.inner
            }

            pub(crate) fn get_mut(&mut self) -> &mut W {
                &mut self.inner
            }

            pub(crate) fn finish(mut self) -> io::Result<W> {
                let compressed = compress_buf(&self.buf, self.level);
                self.inner.write_all(&compressed)?;
                Ok(self.inner)
            }
        }

        impl<W: Write> Write for Encoder<W> {
            fn write(&mut self, data: &[u8]) -> io::Result<usize> {
                self.buf.extend_from_slice(data);
                Ok(data.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        fn compress_buf(buf: &[u8], level: i32) -> Vec<u8> {
            let mut compressed = Vec::new();
            compress(buf, &mut compressed, map_level(level));
            compressed
        }

        fn map_level(level: i32) -> CompressionLevel {
            // ruzstd implements its fastest setting only. Negative zip levels are
            // still compressed frames, so they use that setting too.
            let _ = level;
            CompressionLevel::Fastest
        }
    }
}
