//! Status service.

use crate::AppState;
use actix_web::HttpResponse;

/// Expose liveness status.
///
/// Status:
///  * Live (200 code): The metrics endpoint has started running
///  * Not Live (503 code): everything else.
pub async fn serve_liveness(app_data: actix_web::web::Data<AppState>) -> HttpResponse {
    if app_data.is_live() {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::ServiceUnavailable().finish()
    }
}

/// Expose readiness status.
///
/// Status:
///  * Ready (200 code): the application has been initialized and is available to accept connections.
///  * Not Ready (503 code): no JSON graph available yet.
pub async fn serve_readiness(app_data: actix_web::web::Data<AppState>) -> HttpResponse {
    if app_data.is_ready() {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::ServiceUnavailable().finish()
    }
}
