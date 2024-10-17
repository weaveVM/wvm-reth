use std::num::NonZeroUsize;
use exex_wvm_bigquery::{init_bigquery_db, BigQueryClient, BigQueryConfig};
use std::sync::{Arc, LazyLock};
use tracing::info;
use once_cell::sync::Lazy;

pub static WVM_BIGQUERY: LazyLock<Arc<BigQueryClient>> = LazyLock::new(|| {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async move {
        let config_path: String = std::env::var("CONFIG").unwrap_or_else(|_| "./bq-config.json".to_string());

        info!(target: "wvm::exex","launch config applied from: {}", config_path);

        println!("{}", config_path);

        let config_file = std::fs::File::open(config_path).expect("bigquery config path exists");
        let reader = std::io::BufReader::new(config_file);

        let bq_config: BigQueryConfig =
            serde_json::from_reader(reader).expect("bigquery config read from file");

        let bgc = BigQueryClient::new(&bq_config).await.unwrap();
        //
        // // init bigquery client
        // let bigquery_client =
        //     init_bigquery_db(&bq_config).await.expect("bigquery client initialized");

        info!(target: "wvm::exex", "bigquery client initialized");

        Arc::new(bgc)
    })
});