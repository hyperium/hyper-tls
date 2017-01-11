use std::fmt;
use std::io::{self, Read, Write};

use futures::Async;
use tokio_core::io::Io;
use tokio_core::net::TcpStream;
use tokio_tls::TlsStream;

/// A stream that might be protected with TLS.
pub enum MaybeHttpsStream {
    /// A stream over plain text.
    Http(TcpStream),
    /// A stream protected with TLS.
    Https(TlsStream<TcpStream>),
}

impl fmt::Debug for MaybeHttpsStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MaybeHttpsStream::Http(..) => f.pad("Http(..)"),
            MaybeHttpsStream::Https(..) => f.pad("Https(..)"),
        }
    }
}

impl Read for MaybeHttpsStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match *self {
            MaybeHttpsStream::Http(ref mut s) => s.read(buf),
            MaybeHttpsStream::Https(ref mut s) => s.read(buf),
        }
    }
}

impl Write for MaybeHttpsStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *self {
            MaybeHttpsStream::Http(ref mut s) => s.write(buf),
            MaybeHttpsStream::Https(ref mut s) => s.write(buf),
        }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match *self {
            MaybeHttpsStream::Http(ref mut s) => s.flush(),
            MaybeHttpsStream::Https(ref mut s) => s.flush(),
        }
    }
}

impl Io for MaybeHttpsStream {
    #[inline]
    fn poll_read(&mut self) -> Async<()> {
        match *self {
            MaybeHttpsStream::Http(ref mut s) => s.poll_read(),
            MaybeHttpsStream::Https(ref mut s) => s.poll_read(),
        }
    }

    #[inline]
    fn poll_write(&mut self) -> Async<()> {
        match *self {
            MaybeHttpsStream::Http(ref mut s) => s.poll_write(),
            MaybeHttpsStream::Https(ref mut s) => s.poll_write(),
        }
    }
}
