pub mod error;
mod util;

use error::Result;
use util::str_to_ip;
use warp::{hyper::body::Bytes, Filter};

pub async fn run(addr: &str, port: u16) -> Result<()> {
    let postreceive = warp::path("postreceive")
        .and(warp::post())
        .and(warp::body::bytes())
        .map(|body: Bytes| format!("bytes = {body:?}"));

    let api = warp::path("api").and(postreceive);

    warp::serve(api).run((str_to_ip(addr)?, port)).await;

    Ok(())
}
