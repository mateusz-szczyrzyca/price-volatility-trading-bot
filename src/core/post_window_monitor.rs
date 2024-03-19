use crate::config::settings::ConfigStruct;
use crate::core::calc::percentage_change_between_first_and_last_element;
use rust_decimal::Decimal;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct PostWindowStatus {
    pub symbols_post_window_with_percent_changes: Rc<Cell<HashMap<String, Decimal>>>,
    pub monitor_threshold_reached: bool,
    pub rise_threshold_reached: bool,
    pub drop_threshold_reached: bool,
}

// this function is pure
pub fn calculate_post_window(
    config: ConfigStruct,
    symbol: &str,
    post_window_list: &[Decimal],
    symbols_post_window_with_percent_changes: Rc<Cell<HashMap<String, Decimal>>>,
) -> PostWindowStatus {
    let percent_change_post_window =
        percentage_change_between_first_and_last_element(&post_window_list);

    let mut new_symbols_post_window_map = symbols_post_window_with_percent_changes.take();

    if new_symbols_post_window_map.contains_key(symbol) {
        // there should be only one record in the map per symbol, so we remove old entries to be sure
        new_symbols_post_window_map.remove(symbol);
    }

    new_symbols_post_window_map.insert(symbol.parse().unwrap(), percent_change_post_window);

    let config_rise_min_percent = *config
        .symbol_monitor
        .post_window_price_value_rise_min_max_percent
        .first()
        .unwrap();
    let config_rise_max_percent = *config
        .symbol_monitor
        .post_window_price_value_rise_min_max_percent
        .last()
        .unwrap();

    let config_drop_min_percent = *config
        .symbol_monitor
        .post_window_price_value_drop_min_max_percent
        .first()
        .unwrap();
    let config_drop_max_percent = *config
        .symbol_monitor
        .post_window_price_value_drop_min_max_percent
        .last()
        .unwrap();

    let config_monitor_min_percent = *config
        .symbol_monitor
        .post_window_price_value_monitor_min_max_percent
        .first()
        .unwrap();
    let config_monitor_max_percent = *config
        .symbol_monitor
        .post_window_price_value_monitor_min_max_percent
        .last()
        .unwrap();

    symbols_post_window_with_percent_changes.set(new_symbols_post_window_map);

    let mut post_window_status = PostWindowStatus {
        symbols_post_window_with_percent_changes,
        monitor_threshold_reached: false,
        rise_threshold_reached: false,
        drop_threshold_reached: false,
    };

    // monitor is enabled = both values should not be zero
    if percent_change_post_window >= config_monitor_min_percent
        && percent_change_post_window <= config_monitor_max_percent
    {
        // monitor threshold reached
        post_window_status.monitor_threshold_reached = true;

        if percent_change_post_window >= config_rise_min_percent
            && percent_change_post_window <= config_rise_max_percent
        {
            //
            post_window_status.rise_threshold_reached = true;
        }

        if percent_change_post_window <= config_drop_max_percent
            && percent_change_post_window >= config_drop_min_percent
        {
            //
            post_window_status.drop_threshold_reached = true;
        }
    }

    post_window_status
}
