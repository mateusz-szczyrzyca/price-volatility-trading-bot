use crate::binance::prices::{process_symbol_price, process_symbol_qty};
use crate::binance::trading::{reverse_symbol_action, symbol_buy_or_sell};
use crate::config::settings::ConfigStruct;
use crate::core::calc::{calculate_exit_qty, percent_diff};
use crate::core::structs::OrderBookCommand;
use crate::core::trading::{check_current_profit_percent, TradingSymbol};
use crate::core::types::{
    CurrentTradingProfit, OrderBookCmd, ReadMarketDepthNow, Symbol, SymbolAction, TradingDecision,
    TradingMode, TradingNextStep,
};
use binance::api::Binance;
use binance::market::Market;
use binance::websockets::{WebSockets, WebsocketEvent};
use log::{debug, error, info, warn};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Not;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use tokio::time::Instant;

#[allow(clippy::too_many_arguments)]
pub fn orderbook_executor(
    config: ConfigStruct,
    symbol: Symbol,
    filters_map: HashMap<String, Decimal>,
    symbol_actions: HashMap<Symbol, SymbolAction>,
    my_starting_qty: Decimal,
    monitored_price: Decimal,
    orderbook_cmd: Receiver<OrderBookCommand>,
    driving_signal_out: Sender<TradingSymbol>,
    api_keys: (String, String),
    trading_mode: TradingMode,
) {
    info!("=> starting websocket for: {symbol}");
    let (api_key, secret_key) = api_keys.clone();
    let endpoints =
        [symbol.clone()].map(|symbol| format!("{}@depth@100ms", symbol.to_string().to_lowercase()));

    let (api_key, secret_key) = api_keys.clone();
    let market: Market = Binance::new(Some(api_key.clone()), Some(secret_key.clone()));

    let decimal_zero = Decimal::ZERO;

    // show the information if now is profit (for timeout useful)
    let mut there_is_abs_minimal_profit_now = false;

    let negative_one = Decimal::NEGATIVE_ONE;
    let mut finish_trading_for_symbol_now = false;
    let mut finishing_action_requested = false;

    let mut qty_wanted_to_buy = decimal_zero;
    let mut best_price_now = decimal_zero;

    info!("trading request, symbol: {symbol}, qty: {my_starting_qty}");

    // STATE
    let mut trading_symbol = TradingSymbol {
        symbol: symbol.clone(),
        price: decimal_zero,
        qty: my_starting_qty,
        filters_map,
        current_trading_profit: CurrentTradingProfit::Unknown,
        min_profit_price: decimal_zero,
        good_profit_price: decimal_zero,
        absolute_minimal_profit_percent: decimal_zero,
        trading_started: Instant::now(),
        highest_price_since_min_profit: decimal_zero,
        highest_price_since_good_profit: decimal_zero,
        last_best_price: decimal_zero,
        best_price_now: decimal_zero,
        trading_next_step: TradingNextStep::Join,
        previous_profit_percent: decimal_zero,
        previous_profit_large_change_count: 0,
        trade_decision: TradingDecision::Wait,
        current_symbol_action: symbol_actions.get(&symbol).unwrap().clone(),
        soft_timeout_trading: false,
        current_profit_percent: decimal_zero,
        loss_too_large_displayed: false,
        started_qty: my_starting_qty,
        used_qty: decimal_zero,
        monitored_price,
    };

    trading_symbol.absolute_minimal_profit_percent = config.orderbook_monitor.exchange_comission
        + config
            .orderbook_monitor
            .absolute_minimal_profit_over_comission;

    let final_trade_decision = Cell::new(TradingDecision::Decline);

    loop {
        let mut last_update_id = 0;
        let mut snapshot_taken = false;
        let mut listening_for_orderbook_updates = false;
        // let mut trading_started = Instant::now();
        let mut reading_market_depth_this_time = ReadMarketDepthNow::YES;
        let keep_running = AtomicBool::new(true);
        let final_trade_decision_clone = final_trade_decision.clone();

        let mut web_socket = WebSockets::new(|event: WebsocketEvent| {
            if !snapshot_taken {
                info!("snapshoting orderbook for {symbol}...");
                match market.get_depth(symbol.to_string()) {
                    Ok(answer) => last_update_id = answer.last_update_id,
                    Err(e) => error!("{e:?}"),
                }
                snapshot_taken = true
            }

            if let WebsocketEvent::DepthOrderBook(depth_order_book) = event {
                if listening_for_orderbook_updates {
                    //
                    if depth_order_book.first_update_id == last_update_id + 1 {
                        ///////////////////////////////////////////////////////////////////////////////
                        ///////////////////////// BEGIN: TRADING LOGIC HERE ///////////////////////////
                        ///////////////////////////////////////////////////////////////////////////////
                        last_update_id = depth_order_book.final_update_id;

                        // default values
                        let mut best_ask_price = decimal_zero;
                        let mut best_ask_qty = decimal_zero;

                        //
                        if let Ok(data) = orderbook_cmd.try_recv() {
                            //
                            if data.cmd == OrderBookCmd::StopAndInstantSell {
                                // stop and instant sell everything at current price (limit order)
                                warn!("{symbol} received StopAndInstantSell command");
                                finishing_action_requested = true;
                            }
                        }

                        if finishing_action_requested {
                            finish_trading_for_symbol_now = true;
                        }

                        if trading_symbol.trading_next_step == TradingNextStep::Join
                            && !finishing_action_requested
                        {
                            // only analyse bids if we want to enter
                            for ask in depth_order_book.asks {
                                // first price is the best if we want to buy
                                let ask_price = Decimal::from_f64(ask.price).unwrap();
                                let ask_qty = Decimal::from_f64(ask.qty).unwrap();

                                // binance algorithm: this removes position from orderbook
                                if ask_qty == decimal_zero || ask_price == decimal_zero {
                                    continue;
                                }

                                if best_ask_price == decimal_zero {
                                    // ask should be considered from lowest (best) to highest (worst)
                                    // first price is is the best
                                    best_ask_price = ask_price;
                                    best_ask_qty = ask_qty;
                                }

                                if best_ask_qty >= trading_symbol.qty {
                                    qty_wanted_to_buy = trading_symbol.qty / best_ask_price;

                                    let result = process_symbol_qty(
                                        symbol.clone(),
                                        qty_wanted_to_buy,
                                        &trading_symbol.filters_map,
                                    );

                                    if let Some(val) = result {
                                        let s = trading_symbol.qty;
                                        info!("{symbol} --> starting_qty: {my_starting_qty}, price: {ask_price}, in struct: {s}. qty_wanted_to_buy: {qty_wanted_to_buy}, val: {val}");
                                        qty_wanted_to_buy = val;
                                        best_ask_price = ask_price;
                                        best_ask_qty = ask_qty;

                                        // BEGIN: prevent buy when price diff is too large in comparison with monitor
                                        let price_diff_from_monitor =
                                            percent_diff(monitored_price, best_ask_price).abs();

                                        if price_diff_from_monitor
                                            >= config
                                                .orderbook_monitor
                                                .allowed_buy_diff_from_symbol_monitor_percent
                                        {
                                            trading_symbol.trade_decision =
                                                TradingDecision::Decline;
                                            final_trade_decision_clone
                                                .set(TradingDecision::Decline);
                                            warn!("{symbol} SPREAD REJECTED: price for buy: {best_ask_price}, price from monitor: {monitored_price}, spread: {price_diff_from_monitor}");
                                            break;
                                        }

                                        if trading_symbol.trade_decision != TradingDecision::Decline
                                        {
                                            // ***WARN:*** field modification
                                            trading_symbol.qty = qty_wanted_to_buy;
                                            // ***WARN:*** field modification
                                            trading_symbol.price = best_ask_price;
                                            // ***WARN:*** field modification
                                            trading_symbol.trade_decision = TradingDecision::Start;

                                            final_trade_decision_clone.set(TradingDecision::Start);

                                            // critical - first found then we left
                                            break;
                                        }
                                        // END: prevent buy when price diff is too large in comparison with monitor
                                    }
                                }
                            }
                        }

                        // ask map contains now current asks from lowest to highest - normal iterator is needed

                        let mut best_bid_price = decimal_zero;
                        let mut best_bid_qty = decimal_zero;

                        if trading_symbol.trading_next_step == TradingNextStep::Leave {
                            // only if we want to leave
                            for bids in depth_order_book.bids.iter() {
                                // bids are from highest (best) to lowest (worst)
                                let bid_price = Decimal::from_f64(bids.price).unwrap();
                                let bid_qty = Decimal::from_f64(bids.qty).unwrap();

                                // bids should be consider from highest (best) to lowest (worst), so our best price is the last
                                if bid_price == best_price_now {
                                    //
                                    if bid_qty == decimal_zero {
                                        // https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md
                                        // point 8 - remove price level
                                        best_price_now = decimal_zero;
                                        continue;
                                    }
                                }

                                if bid_qty >= trading_symbol.qty {
                                    //
                                    if bid_price > best_price_now {
                                        // best price init or reset
                                        best_price_now = bid_price;
                                    }
                                    best_bid_price = bid_price;
                                    best_bid_qty = bid_qty;
                                    break;
                                }
                            }
                        }
                        //
                        //
                        //

                        debug!(
                            "{symbol} NOW: best_bid_price: [{best_bid_price}], \
                        best_bid_qty: [{best_bid_qty}], \
                         trade_decision: [{:?}], trading_next_step: [{:?}]",
                            trading_symbol.trade_decision, trading_symbol.trading_next_step
                        );

                        //
                        // BEGIN: starting trading consideration
                        //
                        if trading_symbol.trade_decision == TradingDecision::Start
                            || trading_symbol.trade_decision == TradingDecision::Continue
                        {
                            // first step

                            if trading_symbol.trading_next_step == TradingNextStep::Join {
                                // checking trading possibility
                                //
                                // we can start the trade
                                //
                                if qty_wanted_to_buy > decimal_zero {
                                    // unpack the values - there were already calculated
                                    let my_current_qty = trading_symbol.qty;
                                    let my_current_qty_price = trading_symbol.price;

                                    // log
                                    info!("{symbol} after first step: my_current_qty: {my_current_qty}, my_current_qty_price: {my_current_qty_price}");

                                    // change percent to usable number
                                    let min_percent = config.orderbook_monitor.min_profit_percent
                                        / Decimal::ONE_HUNDRED;

                                    // setting min profit price
                                    // ***WARN:*** field modification
                                    trading_symbol.min_profit_price =
                                        trading_symbol.price + (trading_symbol.price * min_percent);

                                    // None means we can't use this price so stop processing this
                                    // ***WARN:*** field modification
                                    trading_symbol.min_profit_price = process_symbol_price(
                                        trading_symbol.symbol.clone(),
                                        trading_symbol.min_profit_price,
                                        trading_symbol.filters_map.clone(),
                                    )
                                    .unwrap_or_else(|| decimal_zero);

                                    // change percent to usable number
                                    let good_percent = config.orderbook_monitor.good_profit_percent
                                        / Decimal::ONE_HUNDRED;

                                    // setting good profit price
                                    // ***WARN:*** field modification
                                    trading_symbol.good_profit_price = trading_symbol.price
                                        + (trading_symbol.price * good_percent);

                                    // "None" here means we can't use this price so stop trading this pair
                                    // ***WARN:*** field modification
                                    trading_symbol.good_profit_price = match process_symbol_price(
                                        trading_symbol.symbol.clone(),
                                        trading_symbol.good_profit_price,
                                        trading_symbol.filters_map.clone(),
                                    ) {
                                        Some(v) => {
                                            // ***WARN:*** field modification
                                            trading_symbol.trade_decision =
                                                TradingDecision::Continue;

                                            v
                                        }
                                        None => {
                                            // ***WARN:*** field modification
                                            trading_symbol.trade_decision = TradingDecision::Stop;
                                            decimal_zero
                                        }
                                    };

                                    {
                                        let min_profit_price = trading_symbol.min_profit_price;
                                        let good_profit_price = trading_symbol.good_profit_price;
                                        info!("{symbol} after first step: min_profit_price: {min_profit_price}, good_profit_price: {good_profit_price}");
                                    }

                                    final_trade_decision_clone.set(trading_symbol.trade_decision);

                                    if trading_symbol.trade_decision == TradingDecision::Continue {
                                        //
                                        // BEGIN: we are trying to buy asset, that may fail if the price moves too quickly
                                        //
                                        let (a, b) = api_keys.clone();
                                        let (received_qty, used_qty) = symbol_buy_or_sell(
                                            &config,
                                            &trading_mode,
                                            &trading_symbol,
                                            (a, b),
                                        )
                                        .unwrap();

                                        // check if it was done
                                        if received_qty == decimal_zero {
                                            // we could not enter to trade

                                            // ***WARN:*** field modification
                                            trading_symbol.trade_decision = TradingDecision::Stop;
                                            final_trade_decision_clone.set(TradingDecision::Stop);
                                        }
                                        //
                                        // END: we are trying to buy asset, that may fail if the price moves too quickly
                                        //

                                        if trading_symbol.trade_decision
                                            == TradingDecision::Continue
                                        {
                                            // we successfully enter to trade

                                            // ***WARN:*** field modification
                                            trading_symbol.qty = received_qty;
                                            trading_symbol.used_qty = used_qty;

                                            let my_current_qty_price = trading_symbol.price;
                                            let my_current_qty = trading_symbol.qty;
                                            let min_profit_price = trading_symbol.min_profit_price;
                                            let good_profit_price =
                                                trading_symbol.good_profit_price;
                                            info!(
                                        "{symbol} JOINED to TRADE: my_current_qty_price: {my_current_qty_price}, \
                                my_current_qty (received): {my_current_qty}, min_profit_price: {min_profit_price}, \
                                good_profit_price: {good_profit_price}"
                                    );

                                            // reverse symbol action for next action
                                            // ***WARN:*** field modification
                                            trading_symbol.current_symbol_action =
                                                reverse_symbol_action(
                                                    trading_symbol.current_symbol_action.clone(),
                                                );

                                            // we are entering to trade so we have to set some vars
                                            // ***WARN:*** field modification
                                            trading_symbol.trading_next_step =
                                                TradingNextStep::Leave;
                                            // ***WARN:*** field modification
                                            trading_symbol.trading_started = Instant::now();
                                        }
                                    }
                                }
                            }
                            //
                            // END: starting trading consideration
                            //

                            //
                            // BEGIN: TRADING LOGIC
                            //
                            if trading_symbol.trading_next_step == TradingNextStep::Leave
                                && trading_symbol.trade_decision == TradingDecision::Continue
                            {
                                //
                                // price leave calculation algorithms
                                //

                                // default value - it will change later if something is wrong and we
                                // should NOT reading/processing orderbook this time only
                                reading_market_depth_this_time = ReadMarketDepthNow::YES;

                                if best_bid_price == decimal_zero && best_bid_qty == decimal_zero {
                                    // no update - skip reading
                                    reading_market_depth_this_time = ReadMarketDepthNow::NO;
                                }

                                //
                                //
                                //
                                if reading_market_depth_this_time == ReadMarketDepthNow::YES {
                                    // ***WARN:*** field modification
                                    trading_symbol.current_profit_percent =
                                        check_current_profit_percent(
                                            trading_symbol.clone(),
                                            best_ask_qty,
                                            best_ask_price,
                                            best_bid_qty,
                                            best_price_now,
                                        );

                                    //
                                    // BEGIN: difference between last and current profit logic in percent points
                                    //
                                    if trading_symbol.previous_profit_percent
                                        != trading_symbol.current_profit_percent
                                    {
                                        // calculate absolute value no matter if it's profit/loss
                                        if trading_symbol.previous_profit_percent != decimal_zero {
                                            //
                                            // previous_profit_percent is already set
                                            //

                                            // calculate diff between previous and current - we are interested in
                                            // difference, not profit/loss so that why absolute value
                                            let mut larger = trading_symbol.current_profit_percent;
                                            let mut smaller =
                                                trading_symbol.previous_profit_percent;

                                            if larger < trading_symbol.previous_profit_percent {
                                                larger = trading_symbol.previous_profit_percent;
                                                smaller = trading_symbol.current_profit_percent
                                            }
                                            let price_change_diff = larger - smaller;

                                            // previous profit percent is used and should not be zero
                                            if price_change_diff >= config
                                                .orderbook_monitor
                                                .ignore_if_percent_profit_changed_more_than_percent
                                            {
                                                // to big change in percent which should be ignored
                                                if trading_symbol.previous_profit_large_change_count <=
                                                    config.orderbook_monitor.maximum_count_of_profit_changed_ignored_readings {

                                                    // skip reading market data as it's "ignored
                                                    reading_market_depth_this_time = ReadMarketDepthNow::NO;

                                                    // increment number of these ignored readings
                                                    // ***WARN:*** field modification
                                                    trading_symbol.previous_profit_large_change_count += 1;
                                                }

                                                if reading_market_depth_this_time == ReadMarketDepthNow::YES {
                                                    // too many ignored readings - we reset counters
                                                    // and now current reading make as previous

                                                    // ***WARN:*** field modification
                                                    trading_symbol
                                                        .previous_profit_large_change_count = 0;

                                                    // now current reading make as previous because it lasts longer
                                                    // than it expected
                                                    trading_symbol.previous_profit_percent =
                                                        trading_symbol.current_profit_percent;
                                                }
                                            }
                                        }

                                        // first set of previous profit - happens only once
                                        if trading_symbol.previous_profit_percent == decimal_zero {
                                            trading_symbol.previous_profit_percent =
                                                trading_symbol.current_profit_percent
                                        }
                                    }
                                    //
                                    // END: difference between last and current profit logic
                                    //

                                    if best_price_now == decimal_zero {
                                        // no fit orderbook skip this shit to avoid division by zero later
                                        reading_market_depth_this_time = ReadMarketDepthNow::NO;

                                        // just in case - if there is finish action then we have to be sure we sell for something
                                        finish_trading_for_symbol_now = false
                                    }

                                    if trading_symbol.current_profit_percent
                                        < config.orderbook_monitor.loss_limit_sudden_drop_to_percent
                                            * negative_one
                                    {
                                        // no fit orderbook skip this shit to avoid division by zero later
                                        reading_market_depth_this_time = ReadMarketDepthNow::NO;

                                        // just in case - if there is finish action then we have to be sure we sell for something
                                        finish_trading_for_symbol_now = false
                                    }
                                }

                                //
                                // BEGIN: request leave logic (from engine channel)
                                //
                                if finish_trading_for_symbol_now
                                    && reading_market_depth_this_time == ReadMarketDepthNow::YES
                                {
                                    let diff = trading_symbol.trading_started.elapsed();
                                    let minutes = (diff.as_secs() / 60) % 60;
                                    let hours = (diff.as_secs() / 60) / 60;
                                    let time_passed_str = format!("{}h {}m", hours, minutes);

                                    //
                                    // MAIN PROFIT STAT HERE
                                    //
                                    let current_profit_percent =
                                        trading_symbol.current_profit_percent;

                                    let log_prefix = format!("[{current_profit_percent}%] [{time_passed_str}] [{symbol}]");
                                    let exit_qty =
                                        calculate_exit_qty(&config, &trading_symbol).unwrap();

                                    // // //
                                    let my_current_qty = trading_symbol.qty;
                                    let my_current_qty_price = trading_symbol.price;
                                    info!("{log_prefix}: [LEAVE BY REQUEST]
                                        my_used_price: {my_current_qty_price}, best_exit_price: {best_price_now}, exit_qty: {exit_qty} (from my_current_qty: {my_current_qty})");
                                    // // //

                                    // ***WARN:*** field modification
                                    trading_symbol.qty = exit_qty;
                                    // ***WARN:*** field modification
                                    trading_symbol.price = best_price_now;

                                    // so we don't need read market depth anymore as we are finishing now
                                    reading_market_depth_this_time = ReadMarketDepthNow::NO;
                                }
                                //
                                // END: request leave logic (from engine channel)
                                //

                                //
                                //
                                //
                                if reading_market_depth_this_time == ReadMarketDepthNow::YES {
                                    //
                                    // not skipped - we can analyse now as data is ok
                                    //

                                    // ***WARN:*** field modification
                                    trading_symbol.last_best_price = best_price_now;

                                    //
                                    // BEGIN: ABSOLUTE MINIMAL PROFIT
                                    //
                                    there_is_abs_minimal_profit_now = false;

                                    if trading_symbol.current_profit_percent
                                        >= trading_symbol.absolute_minimal_profit_percent
                                    {
                                        there_is_abs_minimal_profit_now = true;
                                    }
                                    //
                                    // END: ABSOLUTE MINIMAL PROFIT
                                    //

                                    // showing trading time for symbol

                                    let diff = trading_symbol.trading_started.elapsed();
                                    let minutes = (diff.as_secs() / 60) % 60;
                                    let hours = (diff.as_secs() / 60) / 60;
                                    let time_passed_str = format!("{}h {}m", hours, minutes);

                                    //
                                    // MAIN PROFIT STAT HERE
                                    //
                                    let my_current_qty = trading_symbol.clone().qty;
                                    let my_current_qty_price = trading_symbol.clone().price;

                                    let current_profit_percent =
                                        trading_symbol.current_profit_percent;

                                    let my_base = trading_symbol.used_qty;
                                    let log_prefix = format!("[{current_profit_percent}%] [{time_passed_str}] [{symbol}]");
                                    info!("{log_prefix}: my price: {my_current_qty_price}, my base: {my_base}, \
                                my qty {my_current_qty}, best price now: {best_price_now} \
                                [NOW: price: {best_bid_price}, qty: {best_bid_qty}]");

                                    //
                                    //
                                    //
                                    //
                                    //

                                    //
                                    // BEGIN: minimal profit
                                    //
                                    if trading_symbol.current_trading_profit
                                        != CurrentTradingProfit::MinimalProfit
                                        && trading_symbol.current_trading_profit
                                            != CurrentTradingProfit::GoodProfit
                                    {
                                        //
                                        // min profit price was crossed but not good profit set?
                                        //
                                        if best_price_now >= trading_symbol.min_profit_price {
                                            info!("{log_prefix}: [__MIN__ PROFIT SET] my_used_price: {my_current_qty_price}, best_price now: {best_price_now}");
                                            // ***WARN:*** field modification
                                            trading_symbol.current_trading_profit =
                                                CurrentTradingProfit::MinimalProfit;
                                            // ***WARN:*** field modification
                                            trading_symbol.highest_price_since_min_profit =
                                                best_price_now;
                                        }
                                    }
                                    //
                                    // END: minimal profit
                                    //

                                    //
                                    // BEGIN: good profit
                                    //
                                    let mut good_profit_reached = false;

                                    if trading_symbol.current_trading_profit
                                        != CurrentTradingProfit::GoodProfit
                                    {
                                        // good profit price?
                                        if best_price_now >= trading_symbol.good_profit_price {
                                            info!("{log_prefix}: [# |GOOD| # PROFIT SET] my_used_price: {my_current_qty_price}, best_price now: {best_price_now}");
                                            // ***WARN:*** field modification
                                            trading_symbol.current_trading_profit =
                                                CurrentTradingProfit::GoodProfit;
                                            // ***WARN:*** field modification
                                            trading_symbol.highest_price_since_good_profit =
                                                best_price_now;
                                            good_profit_reached = true;
                                        }
                                    }

                                    if good_profit_reached {
                                        //
                                        // good profit logic
                                        //
                                        if best_price_now
                                            > trading_symbol.highest_price_since_good_profit
                                        {
                                            let highest_price_since_good_profit = trading_symbol
                                                .clone()
                                                .highest_price_since_good_profit;
                                            // price still rising - remember this
                                            info!("{log_prefix}: [# |GOOD| # PROFIT UPDATE] my_used_price: {my_current_qty_price}, \
                                    previous_highest: {highest_price_since_good_profit}, best_price now: {best_price_now}");
                                            // ***WARN:*** field modification
                                            trading_symbol.highest_price_since_good_profit =
                                                best_price_now;
                                        }

                                        if best_price_now
                                            < trading_symbol.highest_price_since_good_profit
                                        {
                                            // price dropped
                                            let highest_price_since_good_profit = trading_symbol
                                                .clone()
                                                .highest_price_since_good_profit;
                                            let price_drop_now = percent_diff(
                                                best_price_now,
                                                highest_price_since_good_profit,
                                            )
                                            .round_dp_with_strategy(2, RoundingStrategy::ToZero);

                                            if price_drop_now
                                                >= config
                                                    .orderbook_monitor
                                                    .good_profit_crossed_allowed_drop_percent
                                                && best_price_now >= trading_symbol.min_profit_price
                                            {
                                                let exit_qty =
                                                    calculate_exit_qty(&config, &trading_symbol)
                                                        .unwrap();

                                                // // //
                                                let my_current_qty_price = trading_symbol.price;
                                                info!("{log_prefix}: [### |GOOD PROFIT LEAVE| ###] drop: {price_drop_now}%, \
                                        my_used_price: {my_current_qty_price}, best_exit_price: {best_price_now}, exit_qty: {exit_qty} \
                                        [previous_highest: {highest_price_since_good_profit}]");
                                                // // //

                                                // ***WARN:*** field modification
                                                trading_symbol.qty = exit_qty;
                                                // ***WARN:*** field modification
                                                trading_symbol.price = best_price_now;

                                                finish_trading_for_symbol_now = true;
                                            }
                                        }
                                    }
                                    //
                                    // END: good profit
                                    //

                                    //
                                    // BEGIN: min profit
                                    //
                                    if trading_symbol.current_trading_profit
                                        == CurrentTradingProfit::MinimalProfit
                                        && trading_symbol.trade_decision
                                            == TradingDecision::Continue
                                    // only if no signal to leave
                                    {
                                        //
                                        // min profit logic, only if:
                                        // - no good price recorded
                                        // - no stop trading signal
                                        //
                                        let highest_price_since_min_profit =
                                            trading_symbol.clone().highest_price_since_min_profit;
                                        let my_current_qty_price = trading_symbol.price;

                                        if best_price_now > highest_price_since_min_profit {
                                            // price still rising
                                            // log message
                                            info!("{log_prefix}: [_MIN PROFIT UPDATE_] my_used_price: {my_current_qty_price}, \
                                    previous_highest: {highest_price_since_min_profit}, best_price now: {best_price_now}");
                                            // ***WARN:*** field modification
                                            trading_symbol.highest_price_since_min_profit =
                                                best_price_now;
                                        }

                                        if best_price_now < highest_price_since_min_profit {
                                            // price dropped
                                            let percent_drop = percent_diff(
                                                best_price_now,
                                                highest_price_since_min_profit,
                                            )
                                            .round_dp_with_strategy(2, RoundingStrategy::ToZero);

                                            if percent_drop
                                                >= config
                                                    .orderbook_monitor
                                                    .min_profit_crossed_allowed_drop_percent
                                                && best_price_now > trading_symbol.min_profit_price
                                            {
                                                let exit_qty =
                                                    calculate_exit_qty(&config, &trading_symbol)
                                                        .unwrap();

                                                // // //
                                                let highest_price_since_min_profit =
                                                    trading_symbol.highest_price_since_min_profit;
                                                let my_current_qty = trading_symbol.qty;
                                                let my_current_qty_price = trading_symbol.price;
                                                info!("{log_prefix}: [_+++MIN PROFIT LEAVE+++_] drop: {percent_drop}%, \
                                        my_used_price: {my_current_qty_price}, best_exit_price: {best_price_now}, exit_qty: {exit_qty} (from my_current_qty: {my_current_qty}) \
                                        [previous_highest: {highest_price_since_min_profit}]");
                                                // // //

                                                // ***WARN:*** field modification
                                                trading_symbol.qty = exit_qty;
                                                // ***WARN:*** field modification
                                                trading_symbol.price = best_price_now;

                                                finish_trading_for_symbol_now = true;
                                            }
                                        }
                                    }
                                    //
                                    // END: min profit
                                    //

                                    //
                                    // BEGIN: limiting loss logic (when timeout which prevents from instant sells with losses)
                                    //
                                    if config.orderbook_monitor.loss_limit_enabled // loss limiting is enabled in config
                                        && trading_symbol.trade_decision == TradingDecision::Continue // we continue trade
                                        && reading_market_depth_this_time == ReadMarketDepthNow::YES
                                    // we continue reading market depth
                                    {
                                        // loss limit logic kicks in if current_profit_percent is negative
                                        if current_profit_percent < decimal_zero {
                                            let loss_percent = current_profit_percent.abs();
                                            let highest_price_since_min_profit =
                                                trading_symbol.highest_price_since_min_profit;
                                            let my_current_qty_price = trading_symbol.price;
                                            let my_current_qty = trading_symbol.qty;

                                            if loss_percent
                                                > config
                                                    .orderbook_monitor
                                                    .loss_limit_sudden_drop_to_percent
                                            {
                                                // it protects from sudden escape from sudden loss
                                                if trading_symbol.loss_too_large_displayed.not() {
                                                    warn!("{log_prefix}: [LOSS LIMIT IGNORED] my_used_price: {my_current_qty_price}, best_exit_price: {best_price_now}, loss_percent: {loss_percent} (from my_current_qty: {my_current_qty}) [previous_highest: {highest_price_since_min_profit}]");
                                                    // ***WARN:*** field modification
                                                    trading_symbol.loss_too_large_displayed = true;
                                                }

                                                // ***WARN:*** field modification
                                                trading_symbol.current_trading_profit =
                                                    CurrentTradingProfit::LossTooLarge;
                                            }

                                            if loss_percent
                                                >= config.orderbook_monitor.loss_limit_percent
                                                && trading_symbol.current_trading_profit
                                                    == CurrentTradingProfit::LossTooLarge
                                            {
                                                let exit_qty =
                                                    calculate_exit_qty(&config, &trading_symbol)
                                                        .unwrap();

                                                // // //
                                                info!("{log_prefix} [!!! LOSS LIMIT LEAVE !!!]
                                            my_used_price: {my_current_qty_price}, best_exit_price: {best_price_now}, exit_qty: {exit_qty} (from my_current_qty: {my_current_qty}) \
                                            [previous_highest: {highest_price_since_min_profit}]");
                                                // // //

                                                // ***WARN:*** field modification
                                                trading_symbol.qty = exit_qty;
                                                // ***WARN:*** field modification
                                                trading_symbol.price = best_price_now;

                                                finish_trading_for_symbol_now = true;
                                            }
                                        }
                                    }
                                    //
                                    // END: limiting loss logic
                                    //
                                }

                                //
                                // BEGIN: TRADING TIME LIMIT
                                //
                                if (trading_symbol.trading_started.elapsed().as_secs()
                                    >= config.orderbook_monitor.time_limit_secs
                                    || trading_symbol.soft_timeout_trading)
                                    && trading_symbol.trade_decision == TradingDecision::Continue // continue to trade
                                    && reading_market_depth_this_time == ReadMarketDepthNow::YES
                                // continue read market depth
                                {
                                    //
                                    // TIMEOUT ESCAPE
                                    //
                                    let diff = trading_symbol.trading_started.elapsed();
                                    let minutes = (diff.as_secs() / 60) % 60;
                                    let hours = (diff.as_secs() / 60) / 60;
                                    let time_passed_str = format!("{}h {}m", hours, minutes);
                                    let current_profit_percent =
                                        trading_symbol.current_profit_percent;
                                    let log_prefix = format!("[{current_profit_percent}%] [{time_passed_str}] [{symbol}]");

                                    if there_is_abs_minimal_profit_now.not()
                                        && config.orderbook_monitor.time_limit_requires_profit
                                        && trading_symbol.soft_timeout_trading.not()
                                    {
                                        warn!("{log_prefix}: [TIMEOUT EXIT STOPPED] - no profit, so waiting (unless loss_limit will kick in)");
                                    }

                                    if trading_symbol.soft_timeout_trading.not() {
                                        // ***WARN:*** field modification
                                        trading_symbol.soft_timeout_trading = true;
                                    }

                                    let mut best_price = best_price_now;

                                    if best_price == decimal_zero {
                                        best_price = trading_symbol.last_best_price;
                                    }

                                    // division by zero if base is 0
                                    if best_price == decimal_zero {
                                        // warn log
                                    }

                                    //
                                    // BEGIN: possible scenarios when we can stop if timeout
                                    //

                                    let mut we_can_leave_with_profit = false;

                                    /*

                                    Possible scenarios to leave (if timeout):

                                    1) There is good_profit and current price is equal or larger than good_profit_price

                                    2) There is min_profit and current price is equal or larger than min_profit_price
                                       However: it has to be lower than good_profit_price and no good_profit should
                                       be at the same time (good_profit has higher priority)

                                    3) There is absolute minimal profit and current price is equal or larger than
                                       absolute_minimal_profit price. This is the lowest priority scenario.
                                       At the same time we can't have good_profit and min_profit

                                     */

                                    // first we check good profit - highest priority
                                    // if there is good profit - then we can leave with timeout
                                    if trading_symbol.current_trading_profit
                                        == CurrentTradingProfit::GoodProfit
                                        && trading_symbol.good_profit_price <= best_price
                                    {
                                        // there is good profit indeed - we can leave now because of timeout
                                        we_can_leave_with_profit = true;
                                    }

                                    if !we_can_leave_with_profit {
                                        //
                                        // enter here if: NO good_profit
                                        // no good profit - check now min profit, maybe it's applicable
                                        //
                                        if trading_symbol.current_trading_profit
                                            == CurrentTradingProfit::MinimalProfit
                                            && trading_symbol.min_profit_price <= best_price
                                        {
                                            // there is min profit - we can leave now because of timeout
                                            we_can_leave_with_profit = true;
                                        }
                                    }

                                    if !we_can_leave_with_profit {
                                        // we enter here only if: (NO min_profit) AND (NO good_profit)
                                        // we can now check if there is absolute minimal profit
                                        if there_is_abs_minimal_profit_now
                                            || config
                                                .orderbook_monitor
                                                .time_limit_requires_profit
                                                .not()
                                        {
                                            // there is absolute minimal profit or we don't require profit
                                            // so we can leave now because of timeout
                                            we_can_leave_with_profit = true;
                                        }
                                    }

                                    // we_can_leave_with_profit = false means there is timeout and there is NO
                                    // required profit so bot will be waiting

                                    //
                                    // END: possible scenarios when we can stop if timeout
                                    //

                                    if best_price > decimal_zero && we_can_leave_with_profit {
                                        // LEAVE action can happen here after timeout

                                        let exit_qty =
                                            calculate_exit_qty(&config, &trading_symbol).unwrap();

                                        // // //
                                        let highest_price_since_min_profit =
                                            trading_symbol.highest_price_since_min_profit;
                                        let my_used_qty = trading_symbol.qty;
                                        let my_used_price = trading_symbol.price;
                                        info!("{log_prefix}: [TIMEOUT - LEAVE WITH PROFIT] \
                                    my_used_price: {my_used_price}, best_exit_price: {best_price_now}, exit_qty: {exit_qty} (from my_current_qty: {my_used_qty}) \
                                    [previous_highest: {highest_price_since_min_profit}]");
                                        // // //

                                        // ***WARN:*** field modification
                                        trading_symbol.qty = exit_qty;
                                        // ***WARN:*** field modification
                                        trading_symbol.price = best_price_now;

                                        finish_trading_for_symbol_now = true;
                                    }
                                }
                                //
                                // END: TRADING TIME LIMIT
                                //
                            }
                            //
                            // END: TRADING LOGIC
                            //
                        }

                        if finish_trading_for_symbol_now {
                            info!("{symbol}: finishing trading now...");
                            // symbol action
                            let (received_qty, _) = symbol_buy_or_sell(
                                &config,
                                &trading_mode,
                                &trading_symbol,
                                (api_key.clone(), secret_key.clone()),
                            )
                            .unwrap();

                            // ***WARN:*** field modification
                            trading_symbol.qty = received_qty;

                            // finish trading
                            // ***WARN:*** field modification
                            trading_symbol.trade_decision = TradingDecision::Stop;

                            final_trade_decision_clone.set(TradingDecision::Stop);
                        }

                        if trading_symbol.trade_decision == TradingDecision::Decline
                            || trading_symbol.trade_decision == TradingDecision::Stop
                        {
                            driving_signal_out.send(trading_symbol.clone()).unwrap();
                            keep_running.store(false, Ordering::Relaxed);
                        }
                    }

                    ///////////////////////////////////////////////////////////////////////////////
                    ////////////////////////// END: TRADING LOGIC HERE ////////////////////////////
                    ///////////////////////////////////////////////////////////////////////////////
                }

                //
                // BEGIN: not relevant to main logic - this is binance algo for catching orderbook
                //
                if !listening_for_orderbook_updates {
                    //
                    if depth_order_book.final_update_id > last_update_id {
                        //
                        let next_update = last_update_id + 1;
                        if depth_order_book.final_update_id >= next_update {
                            //
                            if depth_order_book.first_update_id <= next_update {
                                last_update_id = depth_order_book.final_update_id;
                                listening_for_orderbook_updates = true;
                            }
                        }
                    }
                }
                //
                // END: not relevant to main logic - this is binance algo for catching orderbook
                //
            }

            Ok(())
        });

        web_socket.connect_multiple_streams(&endpoints).unwrap(); // check error
        let mut it_was_error = false;
        if let Err(e) = web_socket.event_loop(&keep_running) {
            error!("{e:?}");
            it_was_error = true;
        }

        info!("{symbol}: websocket disconnected");

        let _disconnection = web_socket.disconnect();
        if it_was_error && final_trade_decision.get() == TradingDecision::Continue {
            warn!("{symbol} websockect reconnecting as there was an error...");
            continue;
        }

        break;
    }
}
