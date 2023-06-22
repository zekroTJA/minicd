pub mod error;
mod util;

use self::error::ResponseError;
use crate::{config::Config, runner, secrets::SecretManager};
use error::Result;
use std::{convert::Infallible, sync::Arc};
use util::str_to_ip;
use warp::{
    hyper::{body::Bytes, StatusCode},
    Filter, Rejection, Reply,
};

pub async fn run(cfg: &Config, secrets: Arc<SecretManager>) -> Result<()> {
    let postreceive = warp::path("postreceive")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(with_cfg(cfg.clone()))
        .and(with_secrets(secrets.clone()))
        .and_then(handle_postreceive);

    let hello = warp::path("test").and(warp::get()).and_then(handle_test);

    let api = warp::path("api").and(postreceive.or(hello).recover(handle_error));

    warp::serve(api)
        .run((
            str_to_ip(cfg.address.as_deref().unwrap_or("0.0.0.0"))?,
            cfg.port,
        ))
        .await;

    Ok(())
}

fn with_cfg(cfg: Config) -> impl Filter<Extract = (Config,), Error = Infallible> + Clone {
    warp::any().map(move || cfg.clone())
}

fn with_secrets(
    secrets: Arc<SecretManager>,
) -> impl Filter<Extract = (Arc<SecretManager>,), Error = Infallible> + Clone {
    warp::any().map(move || secrets.clone())
}

// See: https://github.com/seanmonstar/warp/blob/master/examples/rejections.rs

async fn handle_error(err: Rejection) -> Result<impl Reply, Infallible> {
    if err.is_not_found() {
        return Ok(warp::reply::with_status(
            "not found".to_string(),
            StatusCode::NOT_FOUND,
        ));
    }

    if let Some(err) = err.find::<ResponseError>() {
        #[allow(clippy::single_match)]
        match err {
            ResponseError::MissingBodyArgs(_) => {
                return Ok(warp::reply::with_status(
                    err.to_string(),
                    StatusCode::BAD_REQUEST,
                ))
            }
            _ => {}
        }
    }

    Ok(warp::reply::with_status(
        format!("{:?}", err),
        StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

// See: https://github.com/seanmonstar/warp/blob/master/examples/todos.rs

async fn handle_postreceive(
    body: Bytes,
    cfg: Config,
    secrets: Arc<SecretManager>,
) -> Result<impl Reply, Rejection> {
    let body = std::str::from_utf8(&body).map_err(ResponseError::InvalidBodyFormat)?;

    let mut args = body.split(' ').filter(|v| !v.is_empty());
    let remote_repo = args
        .next()
        .ok_or(ResponseError::MissingBodyArgs("remote repository"))?;
    let reference = args
        .next()
        .ok_or(ResponseError::MissingBodyArgs("reference parameter"))?;

    runner::run(remote_repo, reference, secrets)
        .await
        .map_err(ResponseError::RunFailed)?;

    Ok(StatusCode::OK)
}

async fn handle_test() -> Result<&'static str, Infallible> {
    Ok("hello")
}
