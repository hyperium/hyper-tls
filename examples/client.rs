extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio_core;

use futures::{Future, Stream};
use std::io::Write;

fn main() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client = hyper::Client::configure()
        .connector(hyper_tls::HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);

    let work = client.get("https://hyper.rs".parse().unwrap()).and_then(|res| {
        println!("Status: {}", res.status());
        println!("Headers:\n{}", res.headers());
        res.body().for_each(|chunk| {
            ::std::io::stdout().write_all(&chunk)
                .map(|_| ())
                .map_err(From::from)
        })
    });
    core.run(work).unwrap();
}
