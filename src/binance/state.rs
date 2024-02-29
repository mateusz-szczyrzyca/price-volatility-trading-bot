use std::collections::HashMap;
use crate::core::types::{Symbol, SymbolAction, TradingMode};
use crate::ConfigStruct;
use rust_decimal::Decimal;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct BinanceState {
    // *********************** BEGIN: basic data types ********************* //
    pub config: ConfigStruct,

    // valid_trading_symbols(): genuine trading symbols taken directly from exchangeInfo.
    pub valid_trading_symbols: Arc<RwLock<HashMap<Symbol, bool>>>,

    // FILTERS MAP, keys:
    // <symbol>-price-filter-min
    // <symbol>-price-filter-max
    // <symbol>-price-filter-tick-size
    // <symbol>-pprice-multi-up
    // <symbol>-pprice-multi-down
    // <symbol>-pprice-avg-price-mins
    // <symbol>-lot-size-min-qty
    // <symbol>-lot-size-max-qty
    // <symbol>-lot-size-step-size
    // <symbol>-min-notional-min
    // <symbol>-min-notional-apply-to-market
    // <symbol>-min-notional-avg-price-mins
    // <symbol>-market-lot-size-min-qty
    // <symbol>-market-lot-size-max-qty
    // <symbol>-market-lot-size-step-size
    pub filters_map: Arc<RwLock<HashMap<String, Decimal>>>,

    // default symbol => action, for BTCUSDT is BUY, but for symbols USDTXXX, it's sell
    pub default_symbol_action: Arc<RwLock<HashMap<Symbol, SymbolAction>>>,

    // for API account
    pub api_key: String,
    pub secret_key: String,

    // trading mode, default is simulation which means no real actions happen
    pub trading_mode: TradingMode,
}
