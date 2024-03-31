use crate::config::settings::ConfigStruct;
use crate::core::post_window_monitor::calculate_post_window;
use crate::core::pre_window_monitor::calculate_pre_window;
use crate::core::types::{SendToTradeDecision, Symbol};
use crate::core::window_monitor::calculate_window;
use binance::websockets::{WebSockets, WebsocketEvent};
use log::{error, info};
use rust_decimal::{Decimal, RoundingStrategy};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Not;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

pub fn all_trades_websocket(
    config: ConfigStruct,
    valid_trading_symbols: Arc<RwLock<HashMap<Symbol, bool>>>,
    channel_to_engine: UnboundedSender<(Symbol, Decimal)>,
) {
    let mut analyze_reminder_shown = false;
    let mut analyze_reminder_time = Instant::now();
    let mut initial_time_passed = false;

    let mut prices_map: HashMap<String, Vec<Decimal>> = HashMap::new();
    let mut symbols_currently_selected_to_monitor: HashMap<String, Decimal> = HashMap::new();
    let mut symbols_pre_window_with_percent_changes: Rc<Cell<HashMap<String, Decimal>>> =
        Rc::new(Cell::new(HashMap::new()));
    let mut symbols_window_with_percent_changes: Rc<Cell<HashMap<String, Decimal>>> =
        Rc::new(Cell::new(HashMap::new()));
    let mut symbols_post_window_with_percent_changes: Rc<Cell<HashMap<String, Decimal>>> =
        Rc::new(Cell::new(HashMap::new()));

    let mut list_valid_symbols = HashMap::new();
    let mut remembered_symbols: HashMap<String, (Instant, Decimal)> = HashMap::new();

    // WARN: the following not used yet
    let mut symbols_already_sent: HashMap<Symbol, bool> = HashMap::new();
    let mut temp_monitored_symbols: HashMap<String, (Instant, Decimal)> = HashMap::new();
    let mut biggest_monitored: HashMap<String, Decimal> = HashMap::new();

    // variability of symbols
    let mut symbols_vars_timestamps: HashMap<String, Instant> = HashMap::new();
    let mut symbols_variability_count: HashMap<String, u64> = HashMap::new();
    let mut symbols_var_last_key: HashMap<String, String> = HashMap::new();

    let keep_running = AtomicBool::new(true); // Used to control the event loop
    let agg_trade = String::from("!ticker@arr");
    loop {
        //
        //
        //
        let mut web_socket = WebSockets::new(|event: WebsocketEvent| {
            if let WebsocketEvent::DayTickerAll(ticker_events) = event {
                {
                    list_valid_symbols = valid_trading_symbols.read().unwrap().clone();
                }

                for tick_event in ticker_events {
                    //
                    // BEGIN: cyclic reminder
                    //
                    if !analyze_reminder_shown {
                        analyze_reminder_shown = true;
                        info!("I'm looking now for volatile pairs...")
                    }

                    if analyze_reminder_time.elapsed().as_secs() >= 300 {
                        analyze_reminder_shown = false;
                        analyze_reminder_time = Instant::now();
                    }
                    //
                    // END: cyclic reminder
                    //

                    let symbol = tick_event.symbol.clone();
                    let symbol_type = Symbol(tick_event.symbol.clone()).clone();

                    if list_valid_symbols.contains_key(&Symbol(symbol.to_string())) {
                        //
                        // this is a legitimate symbol
                        //

                        //
                        // BEGIN: volatility check logic, counts are later available in symbols_var_count map
                        //
                        let current_symbol_var_key = format!(
                            "{}-{}-{}-{}-{}",
                            tick_event.num_trades,
                            tick_event.price_change,
                            tick_event.best_bid,
                            tick_event.best_ask,
                            tick_event.volume
                        );

                        if symbols_variability_count.contains_key(symbol.as_str()) {
                            // maps already contain this symbol

                            // get key of this symbol to compare with new one
                            let previous_key = symbols_var_last_key
                                .get(symbol.clone().as_str())
                                .unwrap()
                                .clone();

                            if previous_key != current_symbol_var_key {
                                // there is a change in volatility - keys mismatch

                                // update key
                                symbols_var_last_key
                                    .insert(symbol.clone(), current_symbol_var_key.clone());

                                // get timestamp
                                let symbol_time_measurement = symbols_vars_timestamps
                                    .get(symbol.clone().as_str())
                                    .unwrap();

                                if symbol_time_measurement.elapsed().as_secs()
                                    >= config.symbol_monitor.symbol_price_violatile_check_time_secs
                                {
                                    // timeout - reset to timer and counts
                                    symbols_variability_count.insert(symbol.clone(), 1);
                                    symbols_vars_timestamps.insert(symbol.clone(), Instant::now());
                                } else {
                                    // still no timeout, still in assessment window - update volatility counter
                                    *symbols_variability_count
                                        .entry(symbol.clone())
                                        .or_insert_with(|| 1) += 1;
                                }
                            }
                        }

                        if !symbols_variability_count.contains_key(symbol.as_str()) {
                            // this symbol is NOT YET present in maps - we add with default values
                            symbols_variability_count.insert(symbol.clone(), 1);
                            symbols_var_last_key.insert(symbol.clone(), current_symbol_var_key);
                            symbols_vars_timestamps.insert(symbol.clone(), Instant::now());
                        }
                        //
                        // END: volatility check logic, counts are later available in symbols_var_count map
                        //

                        let mut old_list: Vec<Decimal> = Vec::new();

                        // current price from stream
                        let current_price =
                            Decimal::from_str(tick_event.best_bid.clone().as_str()).unwrap();

                        if prices_map.contains_key(symbol.as_str()) {
                            old_list = prices_map.get(symbol.as_str()).unwrap().clone();
                        }

                        // construct new list with new added price to the end of the list
                        let new_symbols_percentage_list =
                            add_price_to_list(config.clone(), old_list, current_price);

                        *prices_map
                            .entry(tick_event.symbol.clone())
                            .or_insert_with(|| new_symbols_percentage_list.clone()) =
                            new_symbols_percentage_list.clone();

                        // because we have 3 parts of the list: pre window, window and post window
                        // WARNING: in config we have to have symbol length suitable for
                        // pre/window/post
                        let percentage_change_list_length =
                            config.symbol_monitor.symbol_price_list_length;

                        ////////////////////////////////////////////////////////////////////////
                        ////////////////////////////////////////////////////////////////////////
                        ////////////////////////////////////////////////////////////////////////

                        if new_symbols_percentage_list.len() == percentage_change_list_length {
                            //
                            // we have required count of prices in our list, we can review price changes now
                            // and divide them for pre, main and post window if needed.
                            //
                            if !initial_time_passed {
                                initial_time_passed = true;
                                info!("!!! full symbols lists with prices have been created.")
                            }

                            let all_symbols_length = percentage_change_list_length;

                            let divider = all_symbols_length / 3;

                            // to allow symbol to be sent for trading this is the most important factor
                            let mut symbol_classify_decision = SendToTradeDecision::Negative;

                            //
                            // BEGIN: main window analysis
                            //
                            let main_window = &new_symbols_percentage_list[divider..divider * 2];
                            let window_status = calculate_window(
                                config.clone(),
                                symbol.as_str(),
                                main_window,
                                Rc::clone(&symbols_window_with_percent_changes),
                            );

                            symbols_window_with_percent_changes =
                                window_status.symbols_window_with_percent_changes;

                            if window_status.drop_threshold_reached
                                || window_status.rise_threshold_reached
                            {
                                symbol_classify_decision =
                                    SendToTradeDecision::MainWindowPositiveAnalysis;
                            }

                            //
                            // END: main window_memory
                            //

                            //
                            // BEGIN: pre_window analysis
                            //
                            if config.symbol_monitor.pre_window_analysis
                                && symbol_classify_decision
                                    == SendToTradeDecision::MainWindowPositiveAnalysis
                            {
                                // main window positive decision has to be true, otherwise we don't
                                // analyze this as this is supplement for main window

                                let pre_window = &new_symbols_percentage_list[0..divider];
                                let pre_window_status = calculate_pre_window(
                                    config.clone(),
                                    symbol.as_str(),
                                    pre_window,
                                    Rc::clone(&symbols_pre_window_with_percent_changes),
                                );

                                symbols_pre_window_with_percent_changes =
                                    pre_window_status.symbols_pre_window_with_percent_changes;

                                if pre_window_status.drop_threshold_reached
                                    || pre_window_status.rise_threshold_reached
                                {
                                    symbol_classify_decision =
                                        SendToTradeDecision::MainWindowAndPreWindowPositiveAnalysis;
                                }
                            }
                            //
                            // END: pre_window analysis
                            //

                            //
                            // BEGIN: post window analysis
                            //
                            if config.symbol_monitor.post_window_analysis
                                && symbol_classify_decision
                                    == SendToTradeDecision::MainWindowPositiveAnalysis
                            {
                                let post_window =
                                    &new_symbols_percentage_list[divider..divider * 2];
                                let post_window_status = calculate_post_window(
                                    config.clone(),
                                    symbol.as_str(),
                                    post_window,
                                    Rc::clone(&symbols_window_with_percent_changes),
                                );

                                symbols_post_window_with_percent_changes =
                                    post_window_status.symbols_post_window_with_percent_changes;

                                if post_window_status.drop_threshold_reached
                                    || post_window_status.rise_threshold_reached
                                {
                                    if symbol_classify_decision == SendToTradeDecision::MainWindowAndPreWindowPositiveAnalysis {
                                        symbol_classify_decision = SendToTradeDecision::MainWindowAndBothWindowsPositiveAnalysis;
                                    }

                                    if symbol_classify_decision
                                        == SendToTradeDecision::MainWindowPositiveAnalysis
                                    {
                                        symbol_classify_decision = SendToTradeDecision::MainWindowAndPostWindowPositiveAnalysis;
                                    }
                                }
                            }
                            //
                            // END: post window_memory
                            //

                            match symbol_classify_decision {
                                SendToTradeDecision::MainWindowPositiveAnalysis
                                | SendToTradeDecision::MainWindowAndBothWindowsPositiveAnalysis
                                | SendToTradeDecision::MainWindowAndPreWindowPositiveAnalysis
                                | SendToTradeDecision::MainWindowAndPostWindowPositiveAnalysis => {
                                    // symbol_price_percentages_list.insert(percent_change, tick_event.symbol.clone());
                                    // symbols_currently_selected_to_monitor.insert()
                                    if !symbols_currently_selected_to_monitor.contains_key(&symbol)
                                    {
                                        let s = symbol.clone();
                                        symbols_currently_selected_to_monitor
                                            .insert(s, window_status.percent_change);
                                    }
                                }
                                SendToTradeDecision::Negative => {
                                    if symbols_currently_selected_to_monitor.contains_key(&symbol) {
                                        // this symbol should be removed from the list as now no thresholds were recorded
                                        symbols_currently_selected_to_monitor.remove(&symbol);
                                    }
                                }
                            }
                        }

                        // if (percent_change >= percent_rise_required_to_watch_min
                        //     && percent_change <= percent_rise_required_to_watch_max)
                        //     || (percent_change <= percent_drop_required_to_watch)
                        //     if symbol_classify_decision ==
                        // {
                        //     //
                        //     // we are adding symbols which changes as it crossed our threshold
                        //     //
                        //     add_this_symbol_entry_to_maps = true;
                        //
                        //     if add_this_symbol_entry_to_maps {
                        //         symbol_price_percentages_list
                        //             .insert(percent_change, tick_event.symbol.clone());
                        //     }
                        // }
                        ////////////////////////////////////////////////////////////////////////
                        ////////////////////////////////////////////////////////////////////////
                        ////////////////////////////////////////////////////////////////////////
                    }

                    for (k, v) in symbols_currently_selected_to_monitor.iter() {
                        let key = format!("{}{}", k, v);

                        let event_price = tick_event.best_bid.clone();

                        if remembered_symbols.contains_key(&symbol) {
                            // we have this symbol so we check if it's price needs to be updated
                            let timestamp_now = Instant::now();

                            // take old price from remembered hashmap
                            let (_, old_price) = remembered_symbols.get(symbol.as_str()).unwrap();

                            // new price directly from stream
                            let new_price = Decimal::from_str(&event_price).unwrap();

                            if old_price < &new_price {
                                // if old_price is smaller than new price then refresh this data
                                // in the map and refresh timestamp - symbol which price is rising
                                // will be kept longer in the list
                                let new_tuple = (timestamp_now, *old_price);

                                // update hashmap
                                *remembered_symbols
                                    .entry(tick_event.symbol.clone())
                                    .or_insert_with(|| new_tuple) = new_tuple;
                            }
                        }

                        let price_now = Decimal::from_str(event_price.as_str()).unwrap();

                        if !remembered_symbols.contains_key(symbol.as_str()) {
                            // here we are if we don't know this symbol yet - so add this
                            let val = (
                                Instant::now(),
                                Decimal::from_str(event_price.as_str()).unwrap(),
                            );
                            remembered_symbols.insert(symbol.to_string(), val);
                        }

                        let to_send = (Symbol(k.clone()), price_now);

                        if !symbols_variability_count.contains_key(symbol.as_str()) {
                            // we don't have variability data yet
                            continue;
                        }

                        let volatility_count =
                            *symbols_variability_count.get(symbol.as_str()).unwrap();

                        let percent = v.round_dp_with_strategy(2, RoundingStrategy::ToZero);

                        if volatility_count
                            >= config.symbol_monitor.symbol_price_violatile_required_count
                        {
                            // ############################################################## //
                            // ############################################################## //
                            // ############################################################## //

                            // this symbol has required volatility - so we will send
                            // it now
                            let msg = to_send.clone();
                            info!(
                                    "{symbol}: sent to engine [price: {price_now}, diff: {percent}, volatility count: {volatility_count}]"
                                );
                            channel_to_engine.send(msg).unwrap();

                            symbols_already_sent.insert(symbol_type.clone(), true);
                            let time = Instant::now();
                            temp_monitored_symbols.insert(symbol.clone(), (time, price_now));
                            biggest_monitored.insert(symbol.clone(), price_now);

                            // ############################################################## //
                            // ############################################################## //
                            // ############################################################## //
                        }
                    }
                }
            };
            Ok(())
        });

        web_socket.connect(&agg_trade).unwrap(); // check error
        if let Err(e) = web_socket.event_loop(&keep_running) {
            error!("{e:?}");
        }
        web_socket.disconnect().unwrap();
        info!("symbols monitor disconnected - reconnecting");
    }
}

fn add_price_to_list(config: ConfigStruct, old_vec: Vec<Decimal>, price: Decimal) -> Vec<Decimal> {
    if old_vec.len() < config.symbol_monitor.symbol_price_list_length {
        let mut new_list = old_vec;
        new_list.push(price);
        return new_list;
    }

    let new_value = vec![price];

    if old_vec.is_empty() {
        return new_value;
    }

    let slice = &old_vec[1..old_vec.len()];
    [slice.to_vec(), new_value].concat()
}

fn price_is_constantly_rising(list: Vec<Decimal>) -> bool {
    is_sorted::<Vec<Decimal>>(list)
}

fn is_sorted<T>(data: Vec<Decimal>) -> bool
where
    T: Ord,
{
    data.windows(2).all(|w| w[0] <= w[1])
}
