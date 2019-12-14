/// ConnectorError represents a HttpsConnector error.
pub enum ConnectorError<E: Send> {
    /// An https:// URI was provided when the force_https option was on.
    ForceHttpsButUriNotHttps,
    /// Underlying HttpConnector failed when setting up an HTTP connection.
    HttpConnector(E),
    /// `native_tls` failed when setting up a TLS connection.
    NativeTls(native_tls::Error),
}

impl<E: Send + std::fmt::Debug> std::fmt::Debug for ConnectorError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectorError::ForceHttpsButUriNotHttps => {
                write!(f, "ConnectorError::ForceHttpsButUriNotHttps")
            }
            ConnectorError::HttpConnector(err) => {
                write!(f, "ConnectorError::HttpConnector({:?})", err)
            }
            ConnectorError::NativeTls(err) => write!(f, "ConnectorError::NativeTls({:?})", err),
        }
    }
}

impl<E: Send + std::fmt::Display> std::fmt::Display for ConnectorError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectorError::ForceHttpsButUriNotHttps => {
                write!(f, "https required but URI was not https")
            }
            ConnectorError::HttpConnector(err) => write!(f, "http connector error: {}", err),
            ConnectorError::NativeTls(err) => write!(f, "native tls error: {}", err),
        }
    }
}

impl<E: Send + std::error::Error + 'static> std::error::Error for ConnectorError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConnectorError::ForceHttpsButUriNotHttps => None,
            ConnectorError::HttpConnector(err) => Some(err),
            ConnectorError::NativeTls(err) => Some(err),
        }
    }
}
