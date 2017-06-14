use std::fmt;
use std::io;

use futures::{Future, Poll};
use hyper::client::HttpConnector;
use hyper::Uri;
use native_tls::{TlsConnector, self};
use tokio_core::reactor::Handle;
use tokio_service::Service;
use tokio_tls::TlsConnectorExt;

use stream::MaybeHttpsStream;

/// A builder that creates a TlsConnector
pub trait TlsConnectorBuilder {
    /// Is called by the HttpConnector service to create a TlsConnector
    fn build(&self) -> native_tls::Result<TlsConnector>; 
}

#[derive(Debug)]
pub struct DefaultTlsConnectorBuilder {}

impl TlsConnectorBuilder for DefaultTlsConnectorBuilder {
    fn build(&self) -> native_tls::Result<TlsConnector> {
        TlsConnector::builder().and_then(|c| c.build())
    }
}

/// A Connector for the `https` scheme.
#[derive(Clone)]
pub struct HttpsConnector<Builder = DefaultTlsConnectorBuilder> {
    http: HttpConnector,
    tls_builder: Builder,
}

impl HttpsConnector<DefaultTlsConnectorBuilder> {

    /// Construct a new HttpsConnector.
    ///
    /// Takes number of DNS worker threads.
    pub fn new(threads: usize, handle: &Handle) -> HttpsConnector {
        let mut http = HttpConnector::new(threads, handle);
        http.enforce_http(false);
        HttpsConnector {
            http: http,
            tls_builder: DefaultTlsConnectorBuilder {},
        }
    }
}

impl<Builder: TlsConnectorBuilder> HttpsConnector<Builder> {
    /// Construct a new HttpsConnector with a tls connector builder.
    ///
    /// Takes number of DNS worker threads and the tls connector builder.
    pub fn with_builder(threads: usize,
                        handle: &Handle,
                        builder: Builder) -> HttpsConnector<Builder> {
        let mut http = HttpConnector::new(threads, handle);
        http.enforce_http(false);
        HttpsConnector {
            http: http,
            tls_builder: builder,
        }
    }
}

impl fmt::Debug for HttpsConnector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("HttpsConnector")
            .finish()
    }
}

impl<Builder: TlsConnectorBuilder> Service for HttpsConnector<Builder> {
    type Request = Uri;
    type Response = MaybeHttpsStream;
    type Error = io::Error;
    type Future = HttpsConnecting;

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
        let connecting = self.http.call(uri);
        let tls = self.tls_builder.build();

        HttpsConnecting(if is_https {
            Box::new(connecting.and_then(move |tcp| {
                tls.map(|c| c.connect_async(&host, tcp))
                   .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            }).and_then(|maybe_tls| {
                maybe_tls.map(|tls| MaybeHttpsStream::Https(tls))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            }))
        } else {
            Box::new(connecting.map(|tcp| MaybeHttpsStream::Http(tcp)))
        })
    }

}


/// A Future representing work to connect to a URL, and a TLS handshake.
pub struct HttpsConnecting(Box<Future<Item=MaybeHttpsStream, Error=io::Error>>);


impl Future for HttpsConnecting {
    type Item = MaybeHttpsStream;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

impl fmt::Debug for HttpsConnecting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("HttpsConnecting")
    }
}
