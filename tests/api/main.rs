use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, mpsc};

mod auth_test;
mod e2e_trade_test;

static DB_POOL: OnceLock<PgPool> = OnceLock::new();
static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn get_pool() -> &'static PgPool {
    DB_POOL.get_or_init(|| {
        if std::env::var("JWT_SECRET").is_err() {
            unsafe { std::env::set_var("JWT_SECRET", "test_secret_for_api_tests") };
        }
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost:5432/trading_engine".to_string());

        let (tx, rx) = mpsc::sync_channel::<PgPool>(1);
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build runtime");

            let pool = rt.block_on(async {
                sqlx::postgres::PgPoolOptions::new()
                    .acquire_timeout(std::time::Duration::from_secs(20))
                    .connect(&database_url)
                    .await
                    .expect("failed to connect to database")
            });

            tx.send(pool).expect("channel send failed");
            rt.block_on(std::future::pending::<()>());
        });

        rx.recv().expect("pool init thread panicked")
    })
}

pub fn unique_user(prefix: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}_{id}", prefix)
}
