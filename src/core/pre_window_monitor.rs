use crate::config::settings::ConfigStruct;
use crate::core::calc::percentage_change_between_first_and_last_element;
use rust_decimal::Decimal;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct PreWindowStatus {
    pub symbols_pre_window_with_percent_changes: Rc<Cell<HashMap<String, Decimal>>>,
    pub monitor_threshold_reached: bool,
    pub rise_threshold_reached: bool,
    pub drop_threshold_reached: bool,
}

// this function is pure
pub fn calculate_pre_window(
    config: ConfigStruct,
    symbol: &str,
    pre_window_list: &[Decimal],
    symbols_pre_window_with_percent_changes: Rc<Cell<HashMap<String, Decimal>>>,
) -> PreWindowStatus {
    let percent_change_pre_window =
        percentage_change_between_first_and_last_element(&pre_window_list);

    let mut new_symbols_pre_window_map = symbols_pre_window_with_percent_changes.take();

    if new_symbols_pre_window_map.contains_key(symbol) {
        // there should be only one record in the map per symbol, so we remove old entries to be sure
        new_symbols_pre_window_map.remove(symbol);
    }

    new_symbols_pre_window_map.insert(symbol.parse().unwrap(), percent_change_pre_window);

    let config_rise_min_percent = *config
        .symbol_monitor
        .pre_window_price_value_rise_min_max_percent
        .first()
        .unwrap();
    let config_rise_max_percent = *config
        .symbol_monitor
        .pre_window_price_value_rise_min_max_percent
        .last()
        .unwrap();

    let config_drop_min_percent = *config
        .symbol_monitor
        .pre_window_price_value_drop_min_max_percent
        .first()
        .unwrap();
    let config_drop_max_percent = *config
        .symbol_monitor
        .pre_window_price_value_drop_min_max_percent
        .last()
        .unwrap();

    let config_monitor_min_percent = *config
        .symbol_monitor
        .pre_window_price_value_monitor_min_max_percent
        .first()
        .unwrap();
    let config_monitor_max_percent = *config
        .symbol_monitor
        .pre_window_price_value_monitor_min_max_percent
        .last()
        .unwrap();

    symbols_pre_window_with_percent_changes.set(new_symbols_pre_window_map);

    let mut pre_window_status = PreWindowStatus {
        symbols_pre_window_with_percent_changes,
        monitor_threshold_reached: false,
        rise_threshold_reached: false,
        drop_threshold_reached: false,
    };

    if percent_change_pre_window >= config_monitor_min_percent
        && percent_change_pre_window <= config_monitor_max_percent
    {
        // monitor threshold reached
        pre_window_status.monitor_threshold_reached = true;

        if percent_change_pre_window >= config_rise_min_percent
            && percent_change_pre_window <= config_rise_max_percent
        {
            //
            pre_window_status.rise_threshold_reached = true;
        }

        if percent_change_pre_window <= config_drop_max_percent
            && percent_change_pre_window >= config_drop_min_percent
        {
            //
            pre_window_status.drop_threshold_reached = true;
        }
    }

    pre_window_status
}
