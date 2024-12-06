use exex_wvm_bigquery::{BigQueryClient, BigQueryConfig};
use once_cell::sync::Lazy;
use std::{
    future::Future,
    sync::{Arc, LazyLock},
    time::Instant,
};
use tracing::info;

pub static SUPERVISOR_RT: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread().enable_all().thread_name("wvm").build().unwrap()
});

pub static PRECOMPILE_WVM_BIGQUERY_CLIENT: LazyLock<Arc<BigQueryClient>> = LazyLock::new(|| {
    SUPERVISOR_RT.block_on(
        async move {
            let config_path: String =
                std::env::var("CONFIG").unwrap_or_else(|_| "./bq-config.json".to_string());

            info!(target: "wvm::precompile","precompile big_query config applied from: {}", config_path);

            let config_file =
                std::fs::File::open(config_path).expect("bigquery config path exists");
            let reader = std::io::BufReader::new(config_file);

            let bq_config: BigQueryConfig =
                serde_json::from_reader(reader).expect("bigquery config read from file");

            let bgc = BigQueryClient::new(&bq_config).await.unwrap();

            info!(target: "wvm::precompile", "bigquery client initialized");

            Arc::new(bgc)
        },
    )
});

pub fn internal_block<F: Future>(f: F) -> Result<F::Output, ()> {
    let careful_tokio = std::env::var("CAREFUL_TOKIO").unwrap_or("true".to_string());
    if careful_tokio == "true" {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(r) => r,
            Err(e) => return Err(()),
        };

        Ok(runtime.block_on(f))
    } else {
        info!(target: "wvm::precompile","Non-careful tokio has been called");
        let now = Instant::now();
        let a = Ok(SUPERVISOR_RT.block_on(f));
        println!("Secs to run block {}", now.elapsed().as_secs());
        a
    }
}
