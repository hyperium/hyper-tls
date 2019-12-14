/// HttpsConnectorError represents a HttpsConnector error.
pub enum HttpsConnectorError<E: Send> {
    /// An https:// URI was provided when the force_https option was on.
    ForceHttpsButUriNotHttps,
    /// Underlying HttpConnector failed when setting up an HTTP connection.
    HttpConnector(E),
    /// `native_tls` failed when setting up a TLS connection.
    NativeTls(native_tls::Error),

    #[doc(hidden)]
    __Nonexhaustive,
}

impl<E: Send + std::fmt::Debug> std::fmt::Debug for HttpsConnectorError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpsConnectorError::ForceHttpsButUriNotHttps => {
                f.write_str("HttpsConnectorError::ForceHttpsButUriNotHttps")
            }
            HttpsConnectorError::HttpConnector(err) => f
                .debug_tuple("HttpsConnectorError::HttpConnector")
                .field(err)
                .finish(),
            HttpsConnectorError::NativeTls(err) => f
                .debug_tuple("HttpsConnectorError::NativeTls")
                .field(err)
                .finish(),
            HttpsConnectorError::__Nonexhaustive => unimplemented!(),
        }
    }
}

impl<E: Send + std::fmt::Display> std::fmt::Display for HttpsConnectorError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpsConnectorError::ForceHttpsButUriNotHttps => {
                write!(f, "https required but URI was not https")
            }
            HttpsConnectorError::HttpConnector(err) => write!(f, "http connector error: {}", err),
            HttpsConnectorError::NativeTls(err) => write!(f, "native tls error: {}", err),
            HttpsConnectorError::__Nonexhaustive => unimplemented!(),
        }
    }
}

impl<E: Send + std::error::Error + 'static> std::error::Error for HttpsConnectorError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HttpsConnectorError::ForceHttpsButUriNotHttps => None,
            HttpsConnectorError::HttpConnector(err) => Some(err),
            HttpsConnectorError::NativeTls(err) => Some(err),
            HttpsConnectorError::__Nonexhaustive => unimplemented!(),
        }
    }
}
