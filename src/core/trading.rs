use crate::binance::prices::process_symbol_qty;
use crate::binance::trading::reverse_symbol_action;
use crate::core::calc::percent_diff;
use crate::core::types::{CurrentTradingProfit, Symbol, SymbolAction, TradingDecision, TradingNextStep};
use rust_decimal::{Decimal, RoundingStrategy};
use std::collections::{BTreeMap, HashMap};
use tokio::time::Instant;

#[derive(Debug, Clone)]
pub struct TradingSymbol {
    pub symbol: Symbol,
    pub price: Decimal,
    pub qty: Decimal,
    pub filters_map: HashMap<String, Decimal>,
    pub current_trading_profit: CurrentTradingProfit,
    pub min_profit_price: Decimal,
    pub good_profit_price: Decimal,
    pub absolute_minimal_profit_percent: Decimal,
    pub trading_started: Instant,
    pub highest_price_since_min_profit: Decimal,
    pub highest_price_since_good_profit: Decimal,
    pub last_best_price: Decimal,
    pub best_price_now: Decimal,
    pub trading_next_step: TradingNextStep,
    pub previous_profit_percent: Decimal,
    pub previous_profit_large_change_count: u64,
    pub trade_decision: TradingDecision,
    pub current_symbol_action: SymbolAction,
    pub soft_timeout_trading: bool,
    pub current_profit_percent: Decimal,
    pub loss_too_large_displayed: bool,
    pub started_qty: Decimal,
    pub used_qty: Decimal,
    pub monitored_price: Decimal,
}

// return value: Some(qty, price) - it shows if order action is possible, if yes for what price and qty
// None means it's not possible
#[allow(clippy::too_many_arguments)]
pub fn _symbol_action_qty_price_posibility(
    trading_symbol: TradingSymbol,
    asks_map: BTreeMap<Decimal, Decimal>,
    bids_map: BTreeMap<Decimal, Decimal>,
) -> Option<(Decimal, Decimal)> {
    let mut symbol_action = trading_symbol.current_symbol_action.clone();

    if trading_symbol.trading_next_step == TradingNextStep::Leave {
        symbol_action = reverse_symbol_action(trading_symbol.current_symbol_action);
    }

    // when SIDE=BUY quantity means: I want "quantity" base for this "current_symbol_price"
    if symbol_action == SymbolAction::Buy {
        // check asks from asks.map - lowest is the best
        for ask in asks_map {
            let ask_price = ask.0;
            let ask_qty = ask.1;

            let qty_tmp = trading_symbol.qty / ask_price;
            let my_new_qty_res = process_symbol_qty(
                trading_symbol.symbol.clone(),
                qty_tmp,
                &trading_symbol.filters_map,
            );

            if let Some(my_new_qty) = my_new_qty_res {
                if ask_qty >= my_new_qty {
                    // this is our BUY
                    let tuple = (my_new_qty, ask_price);
                    return Some(tuple);
                }
            }
        }
    }

    // when SIDE=SELL quantity means: I want to use (sell) this my "quantity" for "current_symbol_price"
    if symbol_action == SymbolAction::Sell {
        for bid in bids_map.iter().rev() {
            // bids highest are the best so REVERSE order
            let bid_price = *bid.0;
            let bid_qty = *bid.1;

            let qty_tmp = trading_symbol.qty * bid_price;

            let my_new_qty_res = process_symbol_qty(
                trading_symbol.symbol.clone(),
                qty_tmp,
                &trading_symbol.filters_map,
            );

            if let Some(my_new_qty) = my_new_qty_res {
                if bid_qty >= my_new_qty {
                    // our SELL action
                    let tuple = (my_new_qty, bid_qty);
                    return Some(tuple);
                }
            }
        }
    }

    // None if transaction could have take place
    None
}

#[allow(clippy::too_many_arguments)]
pub fn check_current_profit_percent(
    trading_symbol: TradingSymbol,
    _best_ask_qty: Decimal,
    _best_ask_price: Decimal,
    _best_bid_qty: Decimal,
    best_bid_price: Decimal,
) -> Decimal {
    let decimal_zero = Decimal::ZERO;

    let symbol_action = trading_symbol.current_symbol_action;
    let _symbol = trading_symbol.symbol;

    // when SIDE=BUY quantity means: I want "quantity" base for "current_symbol_price"

    // when SIDE=SELL quantity means: I want to use (sell) this my "quantity" for "current_symbol_price"
    if symbol_action == SymbolAction::Sell {
        let qty_tmp = trading_symbol.qty * best_bid_price;

        return percent_diff(trading_symbol.used_qty, qty_tmp)
            .round_dp_with_strategy(2, RoundingStrategy::ToZero);
    }

    decimal_zero
}
