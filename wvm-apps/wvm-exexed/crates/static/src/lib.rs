use exex_wvm_bigquery::{BigQueryClient, BigQueryConfig};
use std::sync::{Arc, LazyLock};
use tracing::info;

pub static PRECOMPILE_WVM_BIGQUERY_CLIENT: LazyLock<Arc<BigQueryClient>> = LazyLock::new(|| {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
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
