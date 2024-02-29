use crate::binance::object::BinanceObj;
use crate::config::settings::*;
use std::fs;
use std::sync::Arc;

mod binance;
mod config;
mod core;
mod exchange;

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let config_data = fs::read_to_string(CONFIG_FILENAME).expect("cannot read main config file");
    let config: ConfigStruct = toml::from_str(config_data.as_str()).unwrap();
    let binance_exchange = BinanceObj::new(config.clone()).await;
    Arc::new(binance_exchange).start().await;
}
