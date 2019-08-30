//! # hyper-tls
//!
//! An HTTPS connector to be used with [hyper][].
//!
//! [hyper]: https://hyper.rs
//!
//! ## Example
//!
//! ```no_run
//! use hyper_tls::HttpsConnector;
//! use hyper::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), hyper::Error>{
//!     // 4 is number of blocking DNS threads
//!     let https = HttpsConnector::new().unwrap();
//!     let client = Client::builder().build::<_, hyper::Body>(https);
//!
//!     let res = client.get("https://hyper.rs".parse().unwrap()).await?;
//!     assert_eq!(res.status(), 200);
//!     Ok(())
//! }
//! ```
#![doc(html_root_url = "https://docs.rs/hyper-tls/0.3.2")]
#![cfg_attr(test, deny(warnings))]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

pub use client::{Error, HttpsConnecting, HttpsConnector};
pub use stream::{MaybeHttpsStream, TlsStream};

mod client;
mod stream;
