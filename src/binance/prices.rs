use std::collections::HashMap;
use crate::core::types::Symbol;
use log::warn;
use rust_decimal::{Decimal, RoundingStrategy};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct PriceAPIResponse {
    symbol: String,
    price: String,
}

// function provides value for dec.round_dp_with_strategy(<value>, RoundingStrategy::ToZero);
pub fn get_round_number_based_on_ticker(tick_value: Decimal) -> Option<u32> {
    let tick_to_round_map = HashMap::from([
        (dec!(1), 0_u32),
        (dec!(0.1), 1_u32),
        (dec!(0.01), 2_u32),
        (dec!(0.001), 3_u32),
        (dec!(0.0001), 4_u32),
        (dec!(0.00001), 5_u32),
        (dec!(0.000001), 6_u32),
        (dec!(0.0000001), 7_u32),
        (dec!(0.00000001), 8_u32),
    ]);

    if tick_to_round_map.contains_key(&tick_value.normalize()) {
        return Some(*tick_to_round_map.get(&tick_value).unwrap());
    }

    warn!("tick_value: {tick_value}: None (unknown tick value?)");
    None
}

// PRICE_FILTER process
pub fn process_symbol_price(
    symbol: Symbol,
    symbol_price: Decimal,
    filters_map: HashMap<String, Decimal>,
) -> Option<Decimal> {
    let key_price_min = format!("{}-price-filter-min", symbol);
    let key_price_max = format!("{}-price-filter-max", symbol);
    let key_tick_size = format!("{}-price-filter-tick-size", symbol);

    let filter_price_min = *filters_map.get(key_price_min.as_str()).unwrap();
    let filter_price_max = *filters_map.get(key_price_max.as_str()).unwrap();
    let filter_price_tick_size = *filters_map.get(key_tick_size.as_str()).unwrap();

    if filter_price_min > symbol_price {
        warn!("{symbol} => symbol_price: {symbol_price}: filter violation: filter_price_min is not reached: {filter_price_min}");
        return None;
    }

    if filter_price_max < symbol_price {
        warn!("{symbol} => symbol_price: {symbol_price}: filter violation: filter_price_max is smaller than my price: {filter_price_max}");
        return None;
    }

    let round_number = get_round_number_based_on_ticker(filter_price_tick_size);

    if round_number.is_none() {
        warn!("{symbol} => symbol_price: {symbol_price}: filter violation: problem with tick size: {filter_price_tick_size}");
        return None;
    }

    Some(symbol_price.round_dp_with_strategy(round_number.unwrap(), RoundingStrategy::ToZero {}))
}

// LOT_SIZE filter
pub fn process_symbol_qty(
    symbol: Symbol,
    symbol_qty: Decimal,
    filters_map: &HashMap<String, Decimal>,
) -> Option<Decimal> {
    let lot_size_min = format!("{}-lot-size-min-qty", symbol);
    let lot_size_max = format!("{}-lot-size-max-qty", symbol);
    let lot_step_size = format!("{}-lot-size-step-size", symbol);

    let filter_size_min = *filters_map.get(lot_size_min.as_str()).unwrap();
    let filter_size_max = *filters_map.get(lot_size_max.as_str()).unwrap();
    let filter_step_size = *filters_map.get(lot_step_size.as_str()).unwrap();

    if filter_size_min > symbol_qty {
        warn!("{symbol} => symbol_qty: {symbol_qty}: filter violation: filter_size_min is larger than my qty: {filter_size_min}");
        return None;
    }

    if filter_size_max < symbol_qty {
        warn!("{symbol} => symbol_qty: {symbol_qty}: filter violation: filter_size_max is smaller than my qty: {filter_size_max}");
        return None;
    }

    let round_number = get_round_number_based_on_ticker(filter_step_size);
    if round_number.is_none() {
        warn!("{symbol} => symbol_qty: {symbol_qty}: filter violation: problem with filter_step_size: {filter_step_size}");
        return None;
    }

    Some(symbol_qty.round_dp_with_strategy(round_number.unwrap(), RoundingStrategy::ToZero {}))
}
