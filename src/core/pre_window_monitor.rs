use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::config::settings::ConfigStruct;
use crate::core::calc::percentage_change_between_first_and_last_element;
use rust_decimal::Decimal;

pub struct PreWindowStatus {
    pub symbols_prewindow_with_percent_changes: Rc<Cell<HashMap<String, Decimal>>>,
    pub monitor_threshold_reached: bool,
    pub rise_threshold_reached: bool,
    pub drop_threshold_reached: bool,
}

// this function is pure
pub fn calculate_pre_window(
    config: ConfigStruct,
    symbol: String,
    pre_window_list: &[Decimal],
    symbols_prewindow_with_percent_changes: Rc<Cell<HashMap<String, Decimal>>>,
) -> PreWindowStatus {
    let percent_change_prewindow =
        percentage_change_between_first_and_last_element(&pre_window_list);

    let mut new_symbols_prewindow_map = symbols_prewindow_with_percent_changes.take();

    if new_symbols_prewindow_map.contains_key(symbol.as_str()) {
        // there should be only one record in the map per symbol, so we remove old entries to be sure
        new_symbols_prewindow_map.remove(symbol.as_str());
    }

    new_symbols_prewindow_map.insert(symbol.clone(), percent_change_prewindow);

    let rise_min_percent = *config
        .symbol_monitor
        .prewindow_price_value_rise_min_max_percent
        .first()
        .unwrap();
    let rise_max_percent = *config
        .symbol_monitor
        .prewindow_price_value_rise_min_max_percent
        .last()
        .unwrap();

    let drop_min_percent = *config
        .symbol_monitor
        .prewindow_price_value_drop_min_max_percent
        .first()
        .unwrap();
    let drop_max_percent = *config
        .symbol_monitor
        .prewindow_price_value_drop_min_max_percent
        .last()
        .unwrap();

    let monitor_min_percent = *config
        .symbol_monitor
        .prewindow_price_value_monitor_min_max_percent
        .first()
        .unwrap();
    let monitor_max_percent = *config
        .symbol_monitor
        .prewindow_price_value_monitor_min_max_percent
        .last()
        .unwrap();

    symbols_prewindow_with_percent_changes.set(new_symbols_prewindow_map);

    let mut pre_window_status = PreWindowStatus {
        symbols_prewindow_with_percent_changes,
        monitor_threshold_reached: false,
        rise_threshold_reached: false,
        drop_threshold_reached: false,
    };

    if percent_change_prewindow >= monitor_min_percent
        && percent_change_prewindow <= monitor_max_percent
    {
        // monitor threshold reached
        pre_window_status.drop_threshold_reached = true;

        if percent_change_prewindow >= rise_min_percent
            && percent_change_prewindow <= rise_max_percent
        {
            //
            pre_window_status.rise_threshold_reached = true;
        }

        if percent_change_prewindow <= drop_max_percent
            && percent_change_prewindow >= drop_min_percent
        {
            //
            pre_window_status.drop_threshold_reached = true;
        }
    }

    pre_window_status
}
