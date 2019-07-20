use std::fmt;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::future;
use hyper::client::connect::{Connect, Connected, Destination, HttpConnector};
pub use native_tls::Error;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_tls::TlsConnector;

use crate::stream::MaybeHttpsStream;

/// A Connector for the `https` scheme.
#[derive(Clone)]
pub struct HttpsConnector<T> {
    force_https: bool,
    http: T,
    tls: TlsConnector,
}

impl HttpsConnector<HttpConnector> {
    /// Construct a new HttpsConnector.
    ///
    /// Takes number of DNS worker threads.
    ///
    /// This uses hyper's default `HttpConnector`, and default `TlsConnector`.
    /// If you wish to use something besides the defaults, use `From::from`.
    ///
    /// # Note
    ///
    /// By default this connector will use plain HTTP if the URL provded uses
    /// the HTTP scheme (eg: http://example.com/).
    ///
    /// If you would like to force the use of HTTPS then call https_only(true)
    /// on the returned connector.
    pub fn new(threads: usize) -> Result<Self, Error> {
        native_tls::TlsConnector::new().map(|tls| HttpsConnector::new_(threads, tls.into()))
    }

    fn new_(threads: usize, tls: TlsConnector) -> Self {
        let mut http = HttpConnector::new(threads);
        http.enforce_http(false);
        HttpsConnector::from((http, tls))
    }
}

impl<T> HttpsConnector<T> {
    /// Force the use of HTTPS when connecting.
    ///
    /// If a URL is not `https` when connecting, an error is returned.
    pub fn https_only(&mut self, enable: bool) {
        self.force_https = enable;
    }

    #[doc(hidden)]
    #[deprecated(since = "0.3", note = "use `https_only` method instead")]
    pub fn force_https(&mut self, enable: bool) {
        self.force_https = enable;
    }
}

impl<T> From<(T, TlsConnector)> for HttpsConnector<T> {
    fn from(args: (T, TlsConnector)) -> HttpsConnector<T> {
        HttpsConnector {
            force_https: false,
            http: args.0,
            tls: args.1,
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for HttpsConnector<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("HttpsConnector")
            .field("force_https", &self.force_https)
            .field("http", &self.http)
            .finish()
    }
}

impl<T> Connect for HttpsConnector<T>
where
    T: Connect<Error = io::Error>,
    T::Future: 'static,
{
    type Transport = MaybeHttpsStream<T::Transport>;
    type Error = io::Error;
    type Future = HttpsConnecting<T::Transport>;

    fn connect(&self, dst: Destination) -> Self::Future {
        let is_https = dst.scheme() == "https";
        // Early abort if HTTPS is forced but can't be used
        if !is_https && self.force_https {
            let err = io::Error::new(
                io::ErrorKind::Other,
                "HTTPS scheme forced but can't be used",
            );
            return HttpsConnecting(Box::pin(future::err(err)));
        }

        let host = dst.host().to_owned();
        let connecting = self.http.connect(dst);
        let tls = self.tls.clone();
        let fut = async move {
            let (tcp, connected) = match connecting.await {
                Ok(v) => v,
                Err(e) => return Err(e),
            };
            let maybe = if is_https {
                let tls = tls
                    .connect(&host, tcp)
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                MaybeHttpsStream::Https(tls)
            } else {
                MaybeHttpsStream::Http(tcp)
            };
            Ok((maybe, connected))
        };
        HttpsConnecting(Box::pin(fut))
    }
}

type BoxedFut<T> =
    Pin<Box<dyn Future<Output = io::Result<(MaybeHttpsStream<T>, Connected)>> + Send>>;

/// A Future representing work to connect to a URL, and a TLS handshake.
pub struct HttpsConnecting<T>(BoxedFut<T>);

impl<T: AsyncRead + AsyncWrite + Unpin> Future for HttpsConnecting<T> {
    type Output = Result<(MaybeHttpsStream<T>, Connected), io::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}

impl<T> fmt::Debug for HttpsConnecting<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("HttpsConnecting")
    }
}
