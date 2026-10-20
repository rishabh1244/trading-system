use crate::metrics::MetricsCollector;
use actix_web::{HttpResponse, get, web};
use std::sync::Arc;

#[get("/metrics")]
pub async fn get_metrics(metrics: web::Data<Arc<MetricsCollector>>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(metrics.prometheus())
}

#[get("/metrics/json")]
pub async fn get_metrics_json(metrics: web::Data<Arc<MetricsCollector>>) -> HttpResponse {
    let snap = metrics.snapshot();
    HttpResponse::Ok().json(snap)
}
