use std::fmt;
use std::io;
use std::sync::Arc;

use futures::{Future, Poll};
use hyper::client::{Connect, HttpConnector};
use hyper::Uri;
use native_tls::TlsConnector;
use tokio_core::reactor::Handle;
use tokio_service::Service;
use tokio_tls::TlsConnectorExt;

use stream::MaybeHttpsStream;

/// A Connector for the `https` scheme.
#[derive(Clone)]
pub struct HttpsConnector<T> {
    http: T,
    tls: Arc<TlsConnector>,
}

impl HttpsConnector<HttpConnector> {

    /// Construct a new HttpsConnector.
    ///
    /// Takes number of DNS worker threads.
    ///
    /// This uses hyper's default `HttpConnector`, and default `TlsConnector`.
    /// If you wish to use something besides the defaults, use `From::from`.
    pub fn new(threads: usize, handle: &Handle) -> ::native_tls::Result<Self> {
        let mut http = HttpConnector::new(threads, handle);
        http.enforce_http(false);
        let tls = TlsConnector::builder()?.build()?;
        Ok(HttpsConnector::from((http, tls)))
    }
}

impl<T> From<(T, TlsConnector)> for HttpsConnector<T> {
    fn from(args: (T, TlsConnector)) -> HttpsConnector<T> {
        HttpsConnector {
            http: args.0,
            tls: Arc::new(args.1),
        }
    }
}

impl<T> fmt::Debug for HttpsConnector<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("HttpsConnector")
            .finish()
    }
}

impl<T: Connect> Service for HttpsConnector<T> {
    type Request = Uri;
    type Response = MaybeHttpsStream<T::Output>;
    type Error = io::Error;
    type Future = HttpsConnecting<T::Output>;

    fn call(&self, uri: Uri) -> Self::Future {
        let is_https = uri.scheme() == Some("https");
        let host = match uri.host() {
            Some(host) => host.to_owned(),
            None => return HttpsConnecting(
                Box::new(
                    ::futures::future::err(
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "invalid url, missing host"
                        )
                    )
                )
            ),
        };
        let connecting = self.http.connect(uri);
        let tls = self.tls.clone();

        let fut: BoxedFut<T::Output> = if is_https {
            let fut = connecting.and_then(move |tcp| {
                tls.connect_async(&host, tcp)
                    .map(|conn| MaybeHttpsStream::Https(conn))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            });
            Box::new(fut)
        } else {
            Box::new(connecting.map(|tcp| MaybeHttpsStream::Http(tcp)))
        };
        HttpsConnecting(fut)
    }

}

type BoxedFut<T> = Box<Future<Item=MaybeHttpsStream<T>, Error=io::Error>>;

/// A Future representing work to connect to a URL, and a TLS handshake.
pub struct HttpsConnecting<T>(BoxedFut<T>);


impl<T> Future for HttpsConnecting<T> {
    type Item = MaybeHttpsStream<T>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

impl<T> fmt::Debug for HttpsConnecting<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("HttpsConnecting")
    }
}
