use actix_web::{test, web, App};
use std::sync::Arc;
use trading_engine::metrics::MetricsCollector;
use trading_engine::api_gateway::metrics_handler::get_metrics;

fn test_app() -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let collector = Arc::new(MetricsCollector::new());
    App::new()
        .app_data(web::Data::new(collector))
        .service(get_metrics)
}

#[tokio::test]
async fn metrics_empty_returns_zeros() {
    let app = test::init_service(test_app()).await;
    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["order_total"]["count"], 0);
    assert_eq!(body["order_total"]["avg_ms"], 0.0);
    assert_eq!(body["order_total"]["p95_ms"], 0.0);
}

#[tokio::test]
async fn metrics_with_recorded_data() {
    let collector = Arc::new(MetricsCollector::new());
    collector.record_order_total(1000);
    collector.record_order_total(3000);
    collector.record_tx_begin(500);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(collector))
            .service(get_metrics),
    )
    .await;

    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["order_total"]["count"], 2);
    assert!((body["order_total"]["avg_ms"].as_f64().unwrap() - 2.0).abs() < 0.01);
    assert_eq!(body["tx_begin"]["count"], 1);
    assert!((body["tx_begin"]["avg_ms"].as_f64().unwrap() - 0.5).abs() < 0.01);
    assert_eq!(body["reserve_balance"]["count"], 0);
}

#[tokio::test]
async fn concurrent_recording_does_not_panic() {
    use std::thread;

    let collector = Arc::new(MetricsCollector::new());
    let mut handles = vec![];

    for _ in 0..8 {
        let c = collector.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                c.record_order_total(100);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let snap = collector.snapshot();
    assert_eq!(snap.order_total.count, 8000);
    assert!((snap.order_total.avg_ms - 0.1).abs() < 0.01);
}
