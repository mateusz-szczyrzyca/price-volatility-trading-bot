use crate::core::types::Symbol;
use rust_decimal::prelude::*;
use serde::Deserialize;

pub const CONFIG_FILENAME: &str = "config.toml";

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigStruct {
    pub base_starting_assets: Vec<Symbol>,
    pub excluded_symbols: Vec<Symbol>,
    pub excluded_assets: Vec<String>,
    pub symbol_monitor: ConfigSymbolMonitor,
    pub orderbook_monitor: ConfigOrderBookMonitor,
    pub exchange_info_apis: Vec<String>,
    pub exchange_info_fetch_delay_secs: u64,
    pub max_simultaneously_trading_pairs: Decimal,
    pub starting_asset_value: Decimal,
    pub cmd_dir: String,
    pub cmd_read_period_secs: u64,
    pub cmd_stop_and_sell_instantly: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigSymbolMonitor {
    pub symbol_price_list_length: usize,
    pub symbol_price_trigger_percent_value_rise_min: Decimal,
    pub symbol_price_trigger_percent_value_rise_max: Decimal,
    pub symbol_price_trigger_percent_value_drop: Decimal,
    pub symbol_price_trigger_time_period_secs: u64,
    pub symbol_price_trigger_count_within_period: i32,
    pub symbol_price_violatile_check_time_secs: u64,
    pub symbol_price_violatile_required_count: u64,
    pub symbol_stat_list_len: u64,
    pub symbol_stat_list_display_period_secs: u64,
    pub pre_window_analysis: bool,
    pub pre_window_price_value_rise_min_max_percent: [Decimal; 2],
    pub pre_window_price_value_drop_min_max_percent: [Decimal; 2],
    pub pre_window_price_value_monitor_min_max_percent: [Decimal; 2],
    pub window_price_value_rise_min_max_percent: [Decimal; 2],
    pub window_price_value_drop_min_max_percent: [Decimal; 2],
    pub window_price_value_monitor_min_max_percent: [Decimal; 2],
    pub post_window_analysis: bool,
    pub post_window_price_value_rise_min_max_percent: [Decimal; 2],
    pub post_window_price_value_drop_min_max_percent: [Decimal; 2],
    pub post_window_price_value_monitor_min_max_percent: [Decimal; 2],
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigOrderBookMonitor {
    pub allowed_buy_diff_from_symbol_monitor_percent: Decimal,
    pub ignore_if_percent_profit_changed_more_than_percent: Decimal,
    pub maximum_count_of_profit_changed_ignored_readings: u64,
    pub use_profits_to_trade: bool,
    pub acceptable_liquidity_count: Decimal,
    pub exchange_comission: Decimal,
    pub absolute_minimal_profit_over_comission: Decimal,
    pub time_limit_secs: u64,
    pub time_limit_requires_profit: bool,
    pub ultimate_time_limit_enabled: bool,
    pub ultimate_time_limit_secs: u64,
    pub ultimate_time_limit_profit_percent: Decimal,
    pub loss_limit_enabled: bool,
    pub loss_limit_percent: Decimal,
    pub loss_limit_sudden_drop_to_percent: Decimal,
    pub min_profit_percent: Decimal,
    pub min_profit_crossed_allowed_drop_percent: Decimal,
    pub good_profit_percent: Decimal,
    pub good_profit_crossed_allowed_drop_percent: Decimal,
    pub currently_trading_reminder_period_secs: u64,
    pub break_between_trading_same_symbol_secs: u64,
}
