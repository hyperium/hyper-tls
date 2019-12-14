use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::{client::connect::HttpConnector, service::Service, Uri};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tls::TlsConnector;

use crate::error::HttpsConnectorError;
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
    ///
    /// # Panics
    ///
    /// This will panic if the underlying TLS context could not be created.
    ///
    /// To handle that error yourself, you can use the `HttpsConnector::from`
    /// constructor after trying to make a `TlsConnector`.
    pub fn new() -> Self {
        native_tls::TlsConnector::new()
            .map(|tls| HttpsConnector::new_(tls.into()))
            .unwrap_or_else(|e| panic!("HttpsConnector::new() failure: {}", e))
    }

    fn new_(tls: TlsConnector) -> Self {
        let mut http = HttpConnector::new();
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

impl<T> Service<Uri> for HttpsConnector<T>
where
    T: Service<Uri>,
    T::Response: AsyncRead + AsyncWrite + Send + Unpin,
    T::Future: Send + 'static,
    T::Error: Send + Sync + 'static,
{
    type Response = MaybeHttpsStream<T::Response>;
    type Error = HttpsConnectorError<T::Error>;
    type Future = HttpsConnecting<T::Response, Self::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.http.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(HttpsConnectorError::HttpConnector(e))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
        let is_https = dst.scheme_str() == Some("https");
        // Early abort if HTTPS is forced but can't be used
        if !is_https && self.force_https {
            return err(HttpsConnectorError::ForceHttpsButUriNotHttps);
        }

        let host = dst.host().unwrap_or("").to_owned();
        let connecting = self.http.call(dst);
        let tls = self.tls.clone();
        let fut = async move {
            let tcp = connecting
                .await
                .map_err(HttpsConnectorError::HttpConnector)?;
            let maybe = if is_https {
                let tls = tls
                    .connect(&host, tcp)
                    .await
                    .map_err(|e| HttpsConnectorError::NativeTls(e))?;
                MaybeHttpsStream::Https(tls)
            } else {
                MaybeHttpsStream::Http(tcp)
            };
            Ok(maybe)
        };
        HttpsConnecting(Box::pin(fut))
    }
}

fn err<T, E: Send + 'static>(e: E) -> HttpsConnecting<T, E> {
    HttpsConnecting(Box::pin(async { Err(e) }))
}

type BoxedFut<T, E> = Pin<Box<dyn Future<Output = Result<MaybeHttpsStream<T>, E>> + Send>>;

/// A Future representing work to connect to a URL, and a TLS handshake.
pub struct HttpsConnecting<T, E>(BoxedFut<T, E>);

impl<T: AsyncRead + AsyncWrite + Unpin, E> Future for HttpsConnecting<T, E> {
    type Output = Result<MaybeHttpsStream<T>, E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}

impl<T, E> fmt::Debug for HttpsConnecting<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("HttpsConnecting")
    }
}
