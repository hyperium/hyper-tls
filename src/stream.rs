use std::fmt;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use native_tls;
use native_tls::HandshakeError;
use scoped_tls::scoped_thread_local;
use std::future::Future;
use tokio_io::{AsyncRead, AsyncWrite};

scoped_thread_local!(static WAKER: Waker);

#[derive(Debug)]
pub struct SyncStream<S> {
    pub(crate) inner: S,
}

impl<S: Unpin> SyncStream<S> {
    fn with_context<F, R>(&mut self, f: F) -> Result<R, io::Error>
    where
        F: FnOnce(&mut Context, Pin<&mut S>) -> Result<R, io::Error>,
    {
        if !WAKER.is_set() {
            return Err(io::Error::from(io::ErrorKind::WouldBlock));
        }

        WAKER.with(|waker| {
            let cx = &mut Context::from_waker(waker);
            f(cx, Pin::new(&mut self.inner))
        })
    }
}

impl<S: AsyncWrite + Unpin> Write for SyncStream<S> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        self.with_context(|cx, s| match s.poll_write(cx, buf) {
            Poll::Ready(r) => r,
            Poll::Pending => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        })
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        self.with_context(|cx, s| match s.poll_flush(cx) {
            Poll::Ready(r) => r,
            Poll::Pending => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        })
    }
}

impl<S: AsyncRead + Unpin> Read for SyncStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        self.with_context(|cx, s| match s.poll_read(cx, buf) {
            Poll::Ready(r) => r,
            Poll::Pending => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        })
    }
}

/// A stream that might be protected with TLS.
pub enum MaybeHttpsStream<T> {
    /// A stream over plain text.
    Http(T),
    /// A stream protected with TLS.
    Https(TlsStream<T>),
}

/// A stream protected with TLS.
pub struct TlsStream<T> {
    inner: native_tls::TlsStream<SyncStream<T>>,
}

// ===== impl MaybeHttpsStream =====

impl<T: fmt::Debug> fmt::Debug for MaybeHttpsStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MaybeHttpsStream::Http(s) => f.debug_tuple("Http").field(s).finish(),
            MaybeHttpsStream::Https(s) => f.debug_tuple("Https").field(s).finish(),
        }
    }
}

impl<T> From<native_tls::TlsStream<SyncStream<T>>> for MaybeHttpsStream<T> {
    fn from(inner: native_tls::TlsStream<SyncStream<T>>) -> Self {
        MaybeHttpsStream::Https(TlsStream::from(inner))
    }
}

impl<T> From<T> for MaybeHttpsStream<T> {
    fn from(inner: T) -> Self {
        MaybeHttpsStream::Http(inner)
    }
}

impl<T> From<TlsStream<T>> for MaybeHttpsStream<T> {
    fn from(inner: TlsStream<T>) -> Self {
        MaybeHttpsStream::Https(inner)
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> AsyncRead for MaybeHttpsStream<T> {
    #[inline]
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        match self {
            MaybeHttpsStream::Http(s) => s.prepare_uninitialized_buffer(buf),
            MaybeHttpsStream::Https(s) => s.prepare_uninitialized_buffer(buf),
        }
    }

    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, io::Error>> {
        match Pin::get_mut(self) {
            MaybeHttpsStream::Http(s) => Pin::new(s).poll_read(cx, buf),
            MaybeHttpsStream::Https(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl<T: AsyncWrite + AsyncRead + Unpin> AsyncWrite for MaybeHttpsStream<T> {
    #[inline]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match Pin::get_mut(self) {
            MaybeHttpsStream::Http(s) => Pin::new(s).poll_write(cx, buf),
            MaybeHttpsStream::Https(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match Pin::get_mut(self) {
            MaybeHttpsStream::Http(s) => Pin::new(s).poll_shutdown(cx),
            MaybeHttpsStream::Https(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

// ===== impl TlsStream =====

impl<T> TlsStream<T> {
    pub(crate) fn new(inner: native_tls::TlsStream<SyncStream<T>>) -> Self {
        TlsStream { inner }
    }

    /// Get access to the internal `native_tls::TlsStream` stream which also
    /// transitively allows access to `T`.
    pub fn get_ref(&self) -> &native_tls::TlsStream<SyncStream<T>> {
        &self.inner
    }

    /// Get mutable access to the internal `native_tls::TlsStream` stream which
    /// also transitively allows mutable access to `T`.
    pub fn get_mut(&mut self) -> &mut native_tls::TlsStream<SyncStream<T>> {
        &mut self.inner
    }
}

impl<T: fmt::Debug> fmt::Debug for TlsStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<T> From<native_tls::TlsStream<SyncStream<T>>> for TlsStream<T> {
    fn from(stream: native_tls::TlsStream<SyncStream<T>>) -> Self {
        TlsStream { inner: stream }
    }
}

impl<T: AsyncWrite + AsyncRead + Unpin> AsyncRead for TlsStream<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, io::Error>> {
        WAKER.set(cx.waker(), || match Pin::get_mut(self).inner.read(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        })
    }
}

impl<T: AsyncWrite + AsyncRead + Unpin> AsyncWrite for TlsStream<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        WAKER.set(cx.waker(), || match Pin::get_mut(self).inner.write(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        })
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        WAKER.set(cx.waker(), || match Pin::get_mut(self).inner.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        })
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        WAKER.set(cx.waker(), || match Pin::get_mut(self).inner.shutdown() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        })
    }
}

pub struct Handshaking<T> {
    pub(crate) inner: Option<Result<native_tls::TlsStream<T>, HandshakeError<T>>>,
}

impl<T: io::Read + io::Write + Unpin> Future for Handshaking<T> {
    type Output = Result<native_tls::TlsStream<T>, native_tls::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let inner = this.inner.take().expect("polled after ready");
        WAKER.set(cx.waker(), || match inner {
            Ok(stream) => Poll::Ready(Ok(stream.into())),
            Err(HandshakeError::WouldBlock(mid)) => match mid.handshake() {
                Ok(stream) => Poll::Ready(Ok(stream.into())),
                Err(HandshakeError::Failure(err)) => Poll::Ready(Err(err)),
                Err(HandshakeError::WouldBlock(mid)) => {
                    this.inner = Some(Err(HandshakeError::WouldBlock(mid)));
                    Poll::Pending
                }
            },
            Err(HandshakeError::Failure(err)) => Poll::Ready(Err(err)),
        })
    }
}
