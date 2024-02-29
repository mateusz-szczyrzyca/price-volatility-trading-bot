use crate::binance::prices::process_symbol_qty;
use crate::config::settings::ConfigStruct;
use crate::core::trading::TradingSymbol;
use log::info;
pub use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub fn percent_diff(base: Decimal, new: Decimal) -> Decimal {
    let one_hundred = Decimal::ONE_HUNDRED;

    let difference = new - base;

    (difference * one_hundred) / base
}

pub fn calculate_exit_qty(config: &ConfigStruct, trading_symbol: &TradingSymbol) -> Option<Decimal> {
    let comission = config.orderbook_monitor.exchange_comission / Decimal::ONE_HUNDRED;

    let my_current_qty = trading_symbol.qty;
    let exit_qty_tmp = my_current_qty - (my_current_qty * comission);

    let symbol = trading_symbol.symbol.clone();

    let exit_qty_res = process_symbol_qty(symbol.clone(), exit_qty_tmp, &trading_symbol.filters_map);

    if let Some(exit_qty) = exit_qty_res {
        info!("{symbol} calculate exit qty, initial: {my_current_qty}, initial-comission: {exit_qty_tmp}, final exit qty: {exit_qty}");
        return Some(exit_qty);
    }

    None
}

pub fn percentage_change_between_first_and_last_element(list: &&[Decimal]) -> Decimal {
    let before_last = list[0];
    let last = list[list.len() - 1];
    let diff = last - before_last;
    (diff * dec!(100)) / before_last
}
