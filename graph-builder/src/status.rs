//! Status service.

use crate::graph::State;
use actix_web::{HttpRequest, HttpResponse};
use futures::future;
use futures::prelude::*;

/// Expose liveness status.
///
/// Status:
///  * Live (200 code): The upstream scrape loop thread is running
///  * Not Live (500 code): everything else.
pub fn serve_liveness(
    req: HttpRequest,
) -> Box<dyn Future<Item = HttpResponse, Error = failure::Error>> {
    let resp = if req
        .app_data::<State>()
        .expect(commons::MISSING_APPSTATE_PANIC_MSG)
        .is_live()
    {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::InternalServerError().finish()
    };
    Box::new(future::ok(resp))
}

/// Expose readiness status.
///
/// Status:
///  * Ready (200 code): a JSON graph as the result of a successful scrape is available.
///  * Not Ready (500 code): no JSON graph available yet.
pub fn serve_readiness(
    req: HttpRequest,
) -> Box<dyn Future<Item = HttpResponse, Error = failure::Error>> {
    let resp = if req
        .app_data::<State>()
        .expect(commons::MISSING_APPSTATE_PANIC_MSG)
        .is_ready()
    {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::InternalServerError().finish()
    };
    Box::new(future::ok(resp))
}
