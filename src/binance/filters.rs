use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub struct FiltersParser {
    pub data: Vec<Value>,
    pub min_price: Decimal,
    pub max_price: Decimal,
    pub tick_size: Decimal,

    pub percent_price_multi_up: Decimal,
    pub percent_price_multi_down: Decimal,
    pub percent_price_avg_min: Decimal,

    pub lot_min_qty: Decimal,
    pub lot_max_qty: Decimal,
    pub lot_step_size: Decimal,

    pub min_notional_min: Decimal,
    pub notional_apply_to_market: Decimal,
    pub notional_avg_price_mins: Decimal,

    pub market_lot_min_qty: Decimal,
    pub market_lot_max_qty: Decimal,
    pub market_lot_step_size: Decimal,
}

impl FiltersParser {
    pub fn new(v: &[Value]) -> Self {
        let mut min_price_value = dec!(0);
        let mut max_price_value = dec!(0);
        let mut tick_size_value = dec!(0);

        let mut pprice_multi_up_value = dec!(0);
        let mut pprice_multi_down_value = dec!(0);
        let mut pprice_avg_min_value = dec!(0);

        let mut lot_min_qty_value = dec!(0);
        let mut lot_max_qty_value = dec!(0);
        let mut lot_step_size_value = dec!(0);

        let mut notional_min_value = dec!(0);
        let mut notional_apply_to_market_value = dec!(0);
        let mut notional_avg_price_mins_value = dec!(0);

        let mut market_lot_min_qty_value = dec!(0);
        let mut market_lot_max_qty_value = dec!(0);
        let mut market_lot_step_size_value = dec!(0);

        for line in v.iter() {
            let filter_type = &line["filterType"];

            // PRICE_FILTER
            if filter_type.as_str().unwrap() == "PRICE_FILTER" {
                let a = &line["minPrice"];
                if !a.is_null() {
                    min_price_value = Decimal::from_str(a.as_str().unwrap()).unwrap();
                }

                let b = &line["maxPrice"];
                if !b.is_null() {
                    max_price_value = Decimal::from_str(b.as_str().unwrap()).unwrap();
                }

                let c = &line["tickSize"];
                if !c.is_null() {
                    tick_size_value = Decimal::from_str(c.as_str().unwrap()).unwrap();
                }
            }

            // PERCENT_PRICE
            if filter_type.as_str().unwrap() == "PERCENT_PRICE" {
                let a = &line["multiplierUp"];
                if !a.is_null() {
                    pprice_multi_up_value = Decimal::from_str(a.as_str().unwrap()).unwrap();
                }

                let b = &line["multiplierDown"];
                if !b.is_null() {
                    pprice_multi_down_value = Decimal::from_str(b.as_str().unwrap()).unwrap();
                }

                let c = &line["avgPriceMins"];
                if !c.is_null() {
                    pprice_avg_min_value = Decimal::from_i64(c.as_i64().unwrap()).unwrap();
                }
            }

            // LOT_SIZE
            // there has to quantity between minQty and maxQty
            if filter_type.as_str().unwrap() == "LOT_SIZE" {
                let a = &line["minQty"];
                if !a.is_null() {
                    lot_min_qty_value = Decimal::from_str(a.as_str().unwrap()).unwrap();
                }

                let b = &line["maxQty"];
                if !b.is_null() {
                    lot_max_qty_value = Decimal::from_str(b.as_str().unwrap()).unwrap();
                }

                let c = &line["stepSize"];
                if !c.is_null() {
                    lot_step_size_value = Decimal::from_str(c.as_str().unwrap()).unwrap();
                }
            }

            // MIN_NOTIONAL
            if filter_type.as_str().unwrap() == "MIN_NOTIONAL" {
                let a = &line["minNotional"];
                if !a.is_null() {
                    notional_min_value = Decimal::from_str(a.as_str().unwrap()).unwrap();
                }

                let b = &line["applyToMarket"];
                if !b.is_null() {
                    if b.as_bool().unwrap() {
                        notional_apply_to_market_value = dec!(1);
                    } else {
                        notional_apply_to_market_value = dec!(0);
                    }
                }

                let c = &line["avgPriceMins"];
                if !c.is_null() {
                    notional_avg_price_mins_value = Decimal::from_i64(c.as_i64().unwrap()).unwrap();
                }
            }

            // MARKET_LOT_SIZE
            if filter_type.as_str().unwrap() == "MARKET_LOT_SIZE" {
                let a = &line["minQty"];
                if !a.is_null() {
                    market_lot_min_qty_value = Decimal::from_str(a.as_str().unwrap()).unwrap();
                }

                let b = &line["maxQty"];
                if !b.is_null() {
                    market_lot_max_qty_value = Decimal::from_str(b.as_str().unwrap()).unwrap();
                }

                let c = &line["stepSize"];
                if !c.is_null() {
                    market_lot_step_size_value = Decimal::from_str(c.as_str().unwrap()).unwrap();
                }
            }
        }

        Self {
            data: Vec::from(v),
            min_price: min_price_value,
            max_price: max_price_value,
            tick_size: tick_size_value,
            percent_price_multi_up: pprice_multi_up_value,
            percent_price_multi_down: pprice_multi_down_value,
            percent_price_avg_min: pprice_avg_min_value,
            lot_min_qty: lot_min_qty_value,
            lot_max_qty: lot_max_qty_value,
            lot_step_size: lot_step_size_value,
            min_notional_min: notional_min_value,
            notional_apply_to_market: notional_apply_to_market_value,
            notional_avg_price_mins: notional_avg_price_mins_value,
            market_lot_min_qty: market_lot_min_qty_value,
            market_lot_max_qty: market_lot_max_qty_value,
            market_lot_step_size: market_lot_step_size_value,
        }
    }
}
