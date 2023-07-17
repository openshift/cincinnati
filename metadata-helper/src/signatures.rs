use crate::AppState;

use actix_files::NamedFile;
use actix_web::HttpRequest;
use commons::tracing::get_tracer;
use commons::{self, api_response_error, Fallible, GraphError};
use opentelemetry::trace::{mark_span_as_active, Tracer};
use prometheus::{histogram_opts, Histogram, IntCounterVec, Opts, Registry};
use std::collections::HashSet;
use std::path::PathBuf;

lazy_static! {
    static ref SIGNATURES_INCOMING_REQS: IntCounterVec = IntCounterVec::new(
        Opts::new("signatures_incoming_requests_total",
        "Total number of incoming HTTP client request"),
        &["uri_path"]
    )
    .unwrap();
    // Histogram with custom bucket values for serving latency metric (in seconds), values are picked based on monthly data
    static ref SIGNATURES_SERVE_HIST: Histogram = Histogram::with_opts(histogram_opts!(
        "signatures_serve_duration_seconds",
        "HTTP graph serving latency in seconds",
        vec![0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 5.0]
    ))
    .unwrap();
    // Supported Signatures Algo
    static ref SUPPORTED_ALGO: HashSet<&'static str> = HashSet::from(["sha"]);
}

/// Register relevant metrics to a prometheus registry.
pub(crate) fn register_metrics(registry: &Registry) -> Fallible<()> {
    commons::register_metrics(registry)?;
    registry.register(Box::new(SIGNATURES_INCOMING_REQS.clone()))?;
    registry.register(Box::new(SIGNATURES_SERVE_HIST.clone()))?;
    Ok(())
}

/// Serve Cincinnati signatures requests.
pub(crate) async fn index(
    req: HttpRequest,
    app_data: actix_web::web::Data<AppState>,
) -> Result<NamedFile, GraphError> {
    _index(&req, app_data)
        .await
        .map_err(|e| api_response_error(&req, e))
}

async fn _index(
    req: &HttpRequest,
    app_data: actix_web::web::Data<AppState>,
) -> Result<NamedFile, GraphError> {
    let span = get_tracer().start("index");
    let _active_span = mark_span_as_active(span);

    let path = req.uri().path();
    SIGNATURES_INCOMING_REQS.with_label_values(&[path]).inc();

    let timer = SIGNATURES_SERVE_HIST.start_timer();

    let params = req.match_info();
    let algo = params.get("ALGO").unwrap();
    let digest = params.get("DIGEST").unwrap();
    let signature = params.get("SIGNATURE").unwrap();

    if !SUPPORTED_ALGO.contains(algo) {
        timer.observe_duration();
        return Err(GraphError::InvalidParams(format!(
            "algo not supported: {}",
            algo
        )));
    }

    let signatures_data_path = app_data.signatures_dir.clone();
    let mut signature_path = PathBuf::from(signatures_data_path);
    signature_path.push(&format!("{}/{}/{}", algo, digest, signature));

    let f = NamedFile::open(signature_path);
    if f.is_err() {
        timer.observe_duration();
        return Err(GraphError::DoesNotExist(format!(
            "signature does not exist {}",
            f.unwrap_err()
        )));
    }

    timer.observe_duration();
    Ok(f.unwrap())
}
