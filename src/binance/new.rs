use std::collections::HashMap;
use crate::binance::object::BinanceObj;
use crate::binance::state::BinanceState;
use crate::core::cli::determine_bot_trading_mode;
use crate::core::types::{Symbol, SymbolAction, TradingMode};
use crate::ConfigStruct;
use log::info;
use rust_decimal::Decimal;
use std::env;
use std::sync::{Arc, Mutex, RwLock};

impl BinanceObj {
    pub async fn new(config: ConfigStruct) -> Self {
        // this is exposed?
        let valid_trading_symbols: Arc<RwLock<HashMap<Symbol, bool>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // exposed?
        let filters_map: Arc<RwLock<HashMap<String, Decimal>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // symbol actions - some symbols are reversed, like USDT/LOOM
        // so we need map to tell us first action on the symbol, subsequent action
        // will be determined anyway
        let symbol_actions: Arc<RwLock<HashMap<Symbol, SymbolAction>>> =
            Arc::new(RwLock::new(HashMap::new()));

        info!(
            "valid_trading_symbols: {} entries",
            valid_trading_symbols.read().unwrap().len()
        );

        let trading_mode = determine_bot_trading_mode();
        let mut api_key = String::from("");
        let mut secret_key = String::from("");

        if trading_mode == TradingMode::RealTrading {
            let name = "BOT_API_KEY";
            api_key = match env::var(name) {
                Ok(v) => Some(v).unwrap(),
                Err(e) => panic!(
                    "env {} is not set ({}) - it's required to access Binance API",
                    name, e
                ),
            };

            let name = "BOT_SECRET_KEY";
            secret_key = match env::var(name) {
                Ok(v) => Some(v).unwrap(),
                Err(e) => panic!(
                    "env {} is not set ({}) - it's required to access Binance API",
                    name, e
                ),
            };
        }

        info!("Binance object initialized.");

        Self {
            state: Arc::new(Mutex::new(BinanceState {
                config,
                valid_trading_symbols,
                filters_map,
                default_symbol_action: symbol_actions,
                api_key,
                secret_key,
                trading_mode,
            })),
        }
    }
}
