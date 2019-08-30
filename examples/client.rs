use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    let https = hyper_tls::HttpsConnector::new().unwrap();
    let client = hyper::Client::builder().build::<_, hyper::Body>(https);

    let res = client.get("https://hyper.rs".parse().unwrap()).await?;

    println!("Status: {}", res.status());
    println!("Headers:\n{:#?}", res.headers());

    let mut body = res.into_body();
    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        std::io::stdout()
            .write_all(&chunk)
            .expect("example expects stdout to work");
    }
    Ok(())
}
