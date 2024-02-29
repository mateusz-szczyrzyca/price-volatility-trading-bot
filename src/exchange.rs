use std::collections::HashMap;
use crate::core::types::Symbol;
use rust_decimal::Decimal;

pub trait Exchange {
    fn get_all_valid_symbols(&self) -> HashMap<Symbol, bool>;
    // return is (Price, Qty)
    fn process_symbol_price_and_qty(
        &self,
        symbol: Symbol,
        price: Decimal,
        qty: Decimal,
    ) -> (Decimal, Decimal);
}
