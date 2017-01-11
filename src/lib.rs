//! # hyper-tls
//!
//! An HTTPS connector to be used with [hyper][].
//!
//! [hyper]: http://hyper.rs
//!
//! ## Example
//!
//! ```no_run
//! extern crate hyper;
//! extern crate hyper_tls;
//! extern crate tokio_core;
//!
//! fn main() {
//!     let mut core = ::tokio_core::reactor::Core::new().unwrap();
//!
//!     let client = ::hyper::Client::configure()
//!         .connector(::hyper_tls::HttpsConnector::new(4, &core.handle()))
//!         .build(&core.handle());
//!
//!     let res = core.run(client.get("https://www.rust-lang.org".parse().unwrap())).unwrap();
//!     assert_eq!(*res.status(), ::hyper::Ok);
//! }
//! ```
#![deny(warnings)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

extern crate futures;
extern crate hyper;
extern crate native_tls;
extern crate tokio_core;
extern crate tokio_service;
extern crate tokio_tls;

pub use client::HttpsConnector;
pub use stream::MaybeHttpsStream;

mod client;
mod stream;
