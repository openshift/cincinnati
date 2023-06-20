macro_rules! decoder {
    ($(#[$attr:meta])* $name:ident) => {
        pin_project_lite::pin_project! {
            $(#[$attr])*
            #[derive(Debug)]
            ///
            /// This structure implements an [`AsyncRead`](tokio_03::io::AsyncRead) interface and will
            /// read compressed data from an underlying stream and emit a stream of uncompressed data.
            pub struct $name<R> {
                #[pin]
                inner: crate::tokio_03::bufread::Decoder<R, crate::codec::$name>,
            }
        }

        impl<R: tokio_03::io::AsyncBufRead> $name<R> {
            /// Creates a new decoder which will read compressed data from the given stream and
            /// emit a uncompressed stream.
            pub fn new(read: R) -> $name<R> {
                $name {
                    inner: crate::tokio_03::bufread::Decoder::new(read, crate::codec::$name::new()),
                }
            }

            /// Configure multi-member/frame decoding, if enabled this will reset the decoder state
            /// when reaching the end of a compressed member/frame and expect either EOF or another
            /// compressed member/frame to follow it in the stream.
            pub fn multiple_members(&mut self, enabled: bool) {
                self.inner.multiple_members(enabled);
            }

            /// Acquires a reference to the underlying reader that this decoder is wrapping.
            pub fn get_ref(&self) -> &R {
                self.inner.get_ref()
            }

            /// Acquires a mutable reference to the underlying reader that this decoder is
            /// wrapping.
            ///
            /// Note that care must be taken to avoid tampering with the state of the reader which
            /// may otherwise confuse this decoder.
            pub fn get_mut(&mut self) -> &mut R {
                self.inner.get_mut()
            }

            /// Acquires a pinned mutable reference to the underlying reader that this decoder is
            /// wrapping.
            ///
            /// Note that care must be taken to avoid tampering with the state of the reader which
            /// may otherwise confuse this decoder.
            pub fn get_pin_mut(self: std::pin::Pin<&mut Self>) -> std::pin::Pin<&mut R> {
                self.project().inner.get_pin_mut()
            }

            /// Consumes this decoder returning the underlying reader.
            ///
            /// Note that this may discard internal state of this decoder, so care should be taken
            /// to avoid losing resources when this is called.
            pub fn into_inner(self) -> R {
                self.inner.into_inner()
            }
        }

        impl<R: tokio_03::io::AsyncBufRead> tokio_03::io::AsyncRead for $name<R> {
            fn poll_read(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
                buf: &mut tokio_03::io::ReadBuf<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                self.project().inner.poll_read(cx, buf)
            }
        }

        const _: () = {
            fn _assert() {
                use crate::util::{_assert_send, _assert_sync};
                use core::pin::Pin;
                use tokio_03::io::AsyncBufRead;

                _assert_send::<$name<Pin<Box<dyn AsyncBufRead + Send>>>>();
                _assert_sync::<$name<Pin<Box<dyn AsyncBufRead + Sync>>>>();
            }
        };
    }
}
