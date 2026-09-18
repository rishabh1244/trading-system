use actix_web::{get, HttpResponse, web};
use crate::metrics::MetricsCollector;
use std::sync::Arc;

#[get("/metrics")]
pub async fn get_metrics(metrics: web::Data<Arc<MetricsCollector>>) -> HttpResponse {
    let snap = metrics.snapshot();
    HttpResponse::Ok().json(snap)
}
