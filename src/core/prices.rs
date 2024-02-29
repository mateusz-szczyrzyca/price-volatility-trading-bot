use rust_decimal::Decimal;
use std::ops::Div;

pub fn _reverse_price(base_price: Decimal) -> Decimal {
    let one = Decimal::new(1, 0);
    one.div(base_price)
}
