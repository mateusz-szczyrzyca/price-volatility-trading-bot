use crate::config::settings::ConfigStruct;
use crate::core::trading::TradingSymbol;
use crate::core::types::{SymbolAction, TradingMode};
use binance::account::Account;
use binance::api::Binance;
use log::{error, info, warn};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::ops::Not;
use std::process::exit;
use std::{thread, time};

const FILL_BUY_ASK_DELAY: u64 = 2;
const LIMIT_BUY_ATTEMPTS: u64 = 3;
const FILL_SELL_ASK_DELAY: u64 = 10;
const LIMIT_SELL_ATTEMPTS: u64 = 3;

pub fn reverse_symbol_action(symbol_action: SymbolAction) -> SymbolAction {
    if symbol_action == SymbolAction::Buy {
        return SymbolAction::Sell;
    }

    SymbolAction::Buy
}

pub fn symbol_buy_or_sell(
    config: &ConfigStruct,
    trading_mode: &TradingMode,
    trading_symbol: &TradingSymbol,
    api_keys: (String, String),
) -> Result<(Decimal, Decimal), String> {
    // Result<(Decimal, Decimal)> means: (received_qty, used_qty) depends on side
    // default symbol action for Join as we enter
    let decimal_zero = Decimal::ZERO;
    let (api_key, secret_key) = api_keys;
    let binance_account: Account = Binance::new(Some(api_key), Some(secret_key));

    let symbol_string = trading_symbol.symbol.to_string();
    let symbol = trading_symbol.symbol.clone();

    // price and qty which will be used here
    let price = trading_symbol.price.to_f64().unwrap();
    let qty = trading_symbol.qty.to_f64().unwrap();

    if trading_symbol.current_symbol_action == SymbolAction::Buy {
        info!("{symbol} LIMIT BUY (request) => qty (to receive): {qty}, for price: {price}");

        if *trading_mode == TradingMode::Simulation {
            //
            // test order - always success :)
            //
            let qty_to_return = qty;
            let buy_used_qty = price * qty;
            info!(
                "{symbol} [TEST SIMULATION] LIMIT BUY (result) => received qty: [{}]",
                qty_to_return
            );
            let to_return_tuple = (
                Decimal::try_from(qty_to_return).unwrap(),
                Decimal::try_from(buy_used_qty).unwrap(),
            );
            return Ok(to_return_tuple);
        }

        // when SIDE=BUY quantity means: I want "quantity" base for "current_symbol_price"
        match binance_account.limit_buy(symbol_string, qty, price) {
            Err(e) => {
                error!("{symbol} error: {e:?}");
                exit(1);
            }
            Ok(t) => {
                let mut qty_to_return = t.executed_qty;
                let mut buy_used_qty = t.cummulative_quote_qty;
                let mut order_status = t.status;

                let order_id = t.order_id;

                let mut counts = 0;
                let mut order_was_cancelled = false;
                let mut wait_longer_to_confirm = true;

                if order_status == "FILLED" {
                    // filled - finishing
                    info!("{symbol}: OK - order is now successfully filled (INSTANTLY).");
                    wait_longer_to_confirm = false;
                }

                if wait_longer_to_confirm {
                    loop {
                        let order = binance_account
                            .order_status(symbol.0.clone(), t.order_id)
                            .expect("cannot fetch order status");

                        qty_to_return = order.executed_qty.parse().unwrap();
                        buy_used_qty = order.cummulative_quote_qty.parse().unwrap();
                        order_status = order.status;

                        if order_status == "FILLED" {
                            // filled - finishing
                            info!("{symbol}: OK - order is now successfully filled.");
                            break;
                        }

                        if order_was_cancelled && order_status == "PARTIALLY_FILLED" {
                            // cancelled - finishing anyway
                            warn!("{symbol} => ORDER CANCELLED, but partially filled");
                            break;
                        }

                        if order_was_cancelled && qty_to_return == 0.0 {
                            // nothing
                            warn!("{symbol} => ORDER CANCELLED, 0 executed - order unsuccessfull");
                            buy_used_qty = t.cummulative_quote_qty;
                            return Ok((decimal_zero, Decimal::try_from(buy_used_qty).unwrap()));
                        }

                        if counts >= LIMIT_BUY_ATTEMPTS {
                            // first we have to cancell
                            if order_was_cancelled.not() {
                                binance_account
                                    .cancel_order(symbol.to_string(), order_id)
                                    .unwrap();
                                order_was_cancelled = true;
                                warn!("{symbol} => could not make BUY instantly within time limit, CANCELLING ORDER");
                                continue;
                            }
                        }
                        warn!(
                        "{symbol} => BUY: requesting status for order_id={order_id}, status: [{order_status}] [NOT FILLED YET]..."
                    );

                        if order_was_cancelled {
                            break;
                        }
                        counts += 1;
                        thread::sleep(time::Duration::from_secs(FILL_BUY_ASK_DELAY));
                    }
                }

                info!(
                        "{symbol} LIMIT BUY (result) => wanted qty: {}, received qty: [{}], cumm_quote_qty: [{}], price: [{}], status: [{}], side: [{}]",
                        qty, qty_to_return, buy_used_qty, t.price, order_status, t.side,
                    );

                let to_return_tuple = (
                    Decimal::try_from(qty_to_return).unwrap(),
                    Decimal::try_from(buy_used_qty).unwrap(),
                );
                return Ok(to_return_tuple);
            }
        }
    }

    if trading_symbol.current_symbol_action == SymbolAction::Sell {
        // when SIDE=SELL quantity means: I want to USE (sell) this my "quantity" for "current_symbol_price"
        let possible_qty = price * qty;
        info!(
            "{symbol} LIMIT SELL (request) => qty: {qty}, price: {price}, possible qty: {possible_qty}"
        );

        if *trading_mode == TradingMode::Simulation {
            // test sell order - always success :)
            let qty_to_return = qty * price;
            let sell_used_qty = qty;

            info!(
                "{symbol} [TEST SIMULATION] LIMIT SELL (result) => received qty: [{}]",
                qty_to_return
            );

            let to_return_tuple = (
                Decimal::try_from(qty_to_return).unwrap(),
                Decimal::try_from(sell_used_qty).unwrap(),
            );
            return Ok(to_return_tuple);
        }

        match binance_account.limit_sell(symbol_string, qty, price) {
            Err(e) => {
                error!("{symbol} error: {e:?}");
                exit(1);
            }
            Ok(t) => {
                let mut qty_to_return = t.cummulative_quote_qty;
                let sell_used_qty = t.executed_qty;

                if (t.status == "NEW" && t.executed_qty == 0.0) || (t.status == "PARTIALLY_FILLED")
                {
                    // limit order non complete instantly
                    let order_id = t.order_id;
                    let mut count: u64 = 0;
                    loop {
                        let order = binance_account
                            .order_status(symbol.0.clone(), t.order_id)
                            .expect("cannot fetch order status");

                        if order.status == "FILLED" {
                            info!("{symbol}: LEAVE - order is now successfully filled.");
                            qty_to_return = order.cummulative_quote_qty.parse().unwrap();
                            break;
                        }

                        if count >= LIMIT_SELL_ATTEMPTS {
                            warn!("{symbol} LIMIT ORDER still not filled - leaving for later execution");
                            let to_return_tuple = (decimal_zero, decimal_zero);
                            return Ok(to_return_tuple);
                        }

                        warn!("{symbol} => requesting status for order_id={order_id} [NOT FILLED YET]...");
                        count += 1;
                        thread::sleep(time::Duration::from_secs(FILL_SELL_ASK_DELAY));
                    }
                }

                info!("{symbol} LIMIT SELL (result) => to use my qty: {}, qty_to_return (cum) [{}], executed_qty: [{}], price: [{}], status: [{}], side: [{}]",
                        qty, qty_to_return, t.executed_qty, t.price, t.status, t.side);

                let to_return_tuple = (
                    Decimal::try_from(qty_to_return).unwrap(),
                    Decimal::try_from(sell_used_qty).unwrap(),
                );
                return Ok(to_return_tuple);
            }
        }
    }

    let err = "something wrong with symbol action".to_string();
    Err(err)
}
