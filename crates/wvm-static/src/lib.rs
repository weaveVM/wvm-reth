// TODO: move to load network crate and make it subcrate

use load_db::{drivers::planetscale::PlanetScaleDriver, LoadDbConnection};
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

pub static PRECOMPILE_LOADDB_CLIENT: LazyLock<Arc<PlanetScaleDriver>> = LazyLock::new(|| {
    let host = std::env::var("PS_HOST").unwrap_or_default();
    let username = std::env::var("PS_USERNAME").unwrap_or_default();
    let password = std::env::var("PS_PASSWORD").unwrap_or_default();

    let planet_scale_driver = PlanetScaleDriver::new(host, username, password);

    Arc::new(planet_scale_driver)
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
