use std::fmt;
use std::io;
use std::sync::Arc;

use futures::{Future, Poll};
use hyper::client::connect::{Connect, Connected, Destination, HttpConnector};
pub use native_tls::Error;
use native_tls::TlsConnector;
use tokio_tls::TlsConnectorExt;

use stream::MaybeHttpsStream;

/// A Connector for the `https` scheme.
#[derive(Clone)]
pub struct HttpsConnector<T> {
    hostname_verification: bool,
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
    pub fn new(threads: usize) -> Result<Self, Error> {
        let mut http = HttpConnector::new(threads);
        http.enforce_http(false);
        let tls = TlsConnector::builder()?.build()?;
        Ok(HttpsConnector::from((http, tls)))
    }
}

impl<T> HttpsConnector<T> {
    /// Disable hostname verification when connecting.
    ///
    /// # Warning
    ///
    /// You should think very carefully before you use this method. If hostname
    /// verification is not used, any valid certificate for any site will be
    /// trusted for use from any other. This introduces a significant
    /// vulnerability to man-in-the-middle attacks.
    pub fn danger_disable_hostname_verification(&mut self, disable: bool) {
        self.hostname_verification = !disable;
    }
}

impl<T> From<(T, TlsConnector)> for HttpsConnector<T> {
    fn from(args: (T, TlsConnector)) -> HttpsConnector<T> {
        HttpsConnector {
            hostname_verification: true,
            http: args.0,
            tls: Arc::new(args.1),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for HttpsConnector<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("HttpsConnector")
            .field("hostname_verification", &self.hostname_verification)
            .field("http", &self.http)
            .finish()
    }
}

impl<T> Connect for HttpsConnector<T>
where
    T: Connect<Error=io::Error>,
    T::Transport: 'static,
    T::Future: 'static,
{
    type Transport = MaybeHttpsStream<T::Transport>;
    type Error = io::Error;
    type Future = HttpsConnecting<T::Transport>;

    fn connect(&self, dst: Destination) -> Self::Future {
        let is_https = dst.scheme() == "https";
        let host = dst.host().to_owned();

        let connecting = self.http.connect(dst);
        let tls = self.tls.clone();
        let verification = self.hostname_verification;

        let fut: BoxedFut<T::Transport> = if is_https {
            let fut = connecting.and_then(move |(tcp, connected)| {
                let handshake = if verification {
                    tls.connect_async(&host, tcp)
                } else {
                    tls.danger_connect_async_without_providing_domain_for_certificate_verification_and_server_name_indication(tcp)
                };
                handshake
                    .map(|conn| (MaybeHttpsStream::Https(conn), connected))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            });
            Box::new(fut)
        } else {
            Box::new(connecting.map(|(tcp, connected)| {
                (MaybeHttpsStream::Http(tcp), connected)
            }))
        };
        HttpsConnecting(fut)
    }

}

type BoxedFut<T> = Box<Future<Item=(MaybeHttpsStream<T>, Connected), Error=io::Error> + Send>;

/// A Future representing work to connect to a URL, and a TLS handshake.
pub struct HttpsConnecting<T>(BoxedFut<T>);


impl<T> Future for HttpsConnecting<T> {
    type Item = (MaybeHttpsStream<T>, Connected);
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

