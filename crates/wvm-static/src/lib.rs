use exex_wvm_bigquery::{BigQueryClient, BigQueryConfig};
use once_cell::sync::Lazy;
use std::{
    future::Future,
    sync::{Arc, LazyLock, OnceLock},
    time::Instant,
};
use tracing::{error, info};

pub static SUPERVISOR_RT: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread().enable_all().thread_name("wvm").build().unwrap()
});

// Step 2: Create a wrapper struct that will handle lazy initialization
pub struct BigQueryClientManager {
    // The OnceCell ensures initialization happens exactly once
    client: OnceLock<Arc<BigQueryClient>>,
}

impl BigQueryClientManager {
    // Constructor doesn't do any environment variable reading
    pub fn new() -> Self {
        Self { client: OnceLock::new() }
    }

    // This method handles the actual initialization when needed
    pub fn get_client(&self) -> Arc<BigQueryClient> {
        // get_or_init ensures we only initialize once, even with concurrent calls
        self.client
            .get_or_init(|| {
                info!(target: "wvm::static", "Initializing BigQuery client on first use");

                // Use the runtime to run our async initialization
                SUPERVISOR_RT.block_on(async { self.initialize_client().await })
            })
            .clone()
    }

    // Separate method for the actual initialization logic
    async fn initialize_client(&self) -> Arc<BigQueryClient> {
        info!(target: "wvm::static", "Reading BigQuery configuration");

        // Try environment variable first
        if let Ok(env_config) = std::env::var("BIGQUERY_CONFIG") {
            info!(target: "wvm::static", "Found BigQuery config in environment variable");

            match serde_json::from_str::<BigQueryConfig>(&env_config) {
                Ok(bq_config) => {
                    info!(target: "wvm::static", "Successfully parsed BigQuery config from environment");

                    match BigQueryClient::new(&bq_config).await {
                        Ok(client) => {
                            info!(target: "wvm::static", "Successfully initialized BigQuery client from environment");
                            return Arc::new(client)
                        }
                        Err(e) => {
                            panic!(
                                "Failed to initialize BigQuery client from environment config: {e}"
                            );
                        }
                    }
                }
                Err(e) => {
                    panic!("Failed to parse BigQuery config from environment: {e}");
                }
            }
        } else {
            info!(target: "wvm::static", "BigQuery config not found in environment, falling back to file");
        }

        // Fallback to file-based configuration
        let config_path =
            std::env::var("CONFIG").unwrap_or_else(|_| "./bq-config.json".to_string());
        info!(target: "wvm::static", path = %config_path, "Reading BigQuery config from file");

        match std::fs::File::open(&config_path) {
            Ok(file) => {
                let reader = std::io::BufReader::new(file);

                match serde_json::from_reader::<_, BigQueryConfig>(reader) {
                    Ok(bq_config) => match BigQueryClient::new(&bq_config).await {
                        Ok(client) => {
                            info!(target: "wvm::static", "Successfully initialized BigQuery client from file");
                            Arc::new(client)
                        }
                        Err(e) => {
                            panic!("Failed to initialize BigQuery client from file: {e}");
                        }
                    },
                    Err(e) => {
                        panic!("Failed to parse BigQuery config from file: {e}");
                    }
                }
            }
            Err(e) => {
                panic!("Failed to open BigQuery config file: {e}");
            }
        }
    }
}

// Step 3: Create a static manager instance
pub static BQ_CLIENT_MANAGER: Lazy<BigQueryClientManager> =
    Lazy::new(|| BigQueryClientManager::new());

// Step 4: Provide a convenient function to get the client
pub fn get_bigquery_client() -> Arc<BigQueryClient> {
    BQ_CLIENT_MANAGER.get_client()
}

// Replacement for the original PRECOMPILE_WVM_BIGQUERY_CLIENT
// This is a thin wrapper that calls our manager
pub static PRECOMPILE_WVM_BIGQUERY_CLIENT: Lazy<Arc<BigQueryClient>> =
    Lazy::new(|| get_bigquery_client());

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
