use std::fmt;
use std::io;

use futures::{Future, future, Poll};
use hyper::client::connect::{Connect, Connected, Destination, HttpConnector};
pub use native_tls::Error;
use native_tls::{TlsConnector};
use tokio_tls::TlsConnectorExt;

use stream::MaybeHttpsStream;

/// A Connector for the `https` scheme.
pub struct HttpsConnector<T> {
    hostname_verification: bool,
    force_https: bool,
    http: T,
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
    /// By default this connector will use plain HTTP if the URL provded 
    /// uses the HTTP scheme (eg: http://example.com/).
    /// If you would like to force the use of HTTPS
    /// then call force_https(true) on the returned connector.
    pub fn new(threads: usize) -> Result<Self, Error> {
        let mut http = HttpConnector::new(threads);
        http.enforce_http(false);
        Ok(HttpsConnector::from(http))
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

impl<T> HttpsConnector<T> {
    /// Force the use of HTTPS when connecting.
    /// If HTTPS cannot be used the connection will be aborted.
    pub fn force_https(&mut self, enable: bool) {
        self.force_https = enable;
    }
}

impl<T> From<T> for HttpsConnector<T> {
    fn from(arg: T) -> HttpsConnector<T> {
        HttpsConnector {
            hostname_verification: true,
            force_https: false,
            http: arg,
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
    T: Connect<Error=io::Error>,
    T::Transport: 'static,
    T::Future: 'static,
{
    type Transport = MaybeHttpsStream<T::Transport>;
    type Error = io::Error;
    type Future = HttpsConnecting<T::Transport>;

    fn connect(&self, dst: Destination) -> Self::Future {
        let is_https = dst.scheme() == "https";
        // Early abort if HTTPS is forced but can't be used
        if !is_https && self.force_https {
            let err = io::Error::new(io::ErrorKind::Other, "HTTPS scheme forced but can't be used");
            return HttpsConnecting(Box::new(future::err(err)));
        }
        let host = dst.host().to_owned();
        let connecting = self.http.connect(dst);
        let fut: BoxedFut<T::Transport> = if is_https {

            let tls = match TlsConnector::builder()
                .danger_accept_invalid_hostnames(!self.hostname_verification)
                .build() {
                Ok(v)  => v,
                Err(e) => return HttpsConnecting(Box::new(future::err(
                            io::Error::new(io::ErrorKind::Other, e)))),
            };
            let fut = connecting.and_then(move |(tcp, connected)| {
                tls.connect_async(&host, tcp)
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

