use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use crate::config::settings::ConfigStruct;
use crate::core::pre_window_monitor::calculate_pre_window;
use crate::core::types::Symbol;
use binance::websockets::{WebSockets, WebsocketEvent};
use log::{error, info};
use rust_decimal::{Decimal, RoundingStrategy};
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
    let mut symbols_currently_selected_to_monitor: HashMap<Decimal, String> = HashMap::new();
    let mut symbols_prewindow_with_percent_changes: Rc<Cell<HashMap<String, Decimal>>> = Rc::new(Cell::new(HashMap::new()));

    let mut symbols_displayed: HashMap<String, bool> = HashMap::new();
    let mut list_valid_symbols = HashMap::new();
    let mut remembered_symbols: HashMap<String, (Instant, Decimal)> = HashMap::new();
    let mut symbols_sent: HashMap<Symbol, bool> = HashMap::new();

    let mut temp_monitored_symbols: HashMap<String, (Instant, Decimal)> = HashMap::new();
    let mut biggest_monitored: HashMap<String, Decimal> = HashMap::new();

    // variability of symbols
    let mut symbols_vars_timestamps: HashMap<String, Instant> = HashMap::new();
    let mut symbols_var_count: HashMap<String, u64> = HashMap::new();
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

                        if symbols_var_count.contains_key(symbol.as_str()) {
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
                                    symbols_var_count.insert(symbol.clone(), 1);
                                    symbols_vars_timestamps.insert(symbol.clone(), Instant::now());
                                } else {
                                    // still no timeout, still in assessment window - update volatility counter
                                    *symbols_var_count
                                        .entry(symbol.clone())
                                        .or_insert_with(|| 1) += 1;
                                }
                            }
                        }

                        if symbols_var_count.contains_key(symbol.as_str()).not() {
                            // this symbol is NOT YET present in maps - we add with default values
                            symbols_var_count.insert(symbol.clone(), 1);
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

                        if new_symbols_percentage_list.len() == percentage_change_list_length {
                            //
                            // we have required count of prices in our list, we can review price changes now
                            //
                            if initial_time_passed.not() {
                                initial_time_passed = true;
                                info!("!!! full symbols lists with prices have been created.")
                            }

                            let all = percentage_change_list_length;

                            let divider = all / 3;

                            let pre_window = &new_symbols_percentage_list[0..divider];
                            let window = &new_symbols_percentage_list[divider..divider * 2];
                            let post_window = &new_symbols_percentage_list[divider * 2..all];

                            let this_symbol_may_be_sent = false;

                            //
                            // BEGIN: pre_window memory
                            // TODO:
                            let pre_window_status = calculate_pre_window(
                                config.clone(),
                                symbol.clone(),
                                pre_window,
                                Rc::clone(&symbols_prewindow_with_percent_changes),
                            );

                            symbols_prewindow_with_percent_changes =
                                pre_window_status.symbols_prewindow_with_percent_changes;

                            if pre_window_status.drop_threshold_reached {
                                info!("pre window status drop threshold reached!")
                            }

                            if pre_window_status.rise_threshold_reached {
                                info!("pre window status rise threshold reached!")
                            }
                            //
                            // END; pre_window_memory
                            //

                            // let add_this_symbol_entry_to_maps;

                            // if (percent_change >= percent_rise_required_to_watch_min
                            //     && percent_change <= percent_rise_required_to_watch_max)
                            //     || (percent_change <= percent_drop_required_to_watch)
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
                        }

                        for (k, v) in symbols_currently_selected_to_monitor.iter() {
                            let key = format!("{}{}", v, k);

                            if !symbols_displayed.contains_key(key.as_str()) {
                                let event_price = tick_event.best_bid.clone();

                                if remembered_symbols.contains_key(symbol.clone().as_str()) {
                                    let timestamp_now = Instant::now();
                                    let (_, old_price) =
                                        remembered_symbols.get(symbol.as_str()).unwrap();
                                    let new_price = Decimal::from_str(&event_price).unwrap();
                                    if old_price < &new_price {
                                        // if old_price is smaller than new price then refresh this data
                                        // in the map and refresh timestamp - symbol which price is rising
                                        // will be kept longer in the list
                                        let new_tuple = (timestamp_now, *old_price);

                                        *remembered_symbols
                                            .entry(tick_event.symbol.clone())
                                            .or_insert_with(|| new_tuple) = new_tuple;
                                    }
                                }

                                if !remembered_symbols.contains_key(symbol.as_str()) {
                                    let v = (
                                        Instant::now(),
                                        Decimal::from_str(event_price.as_str()).unwrap(),
                                    );
                                    remembered_symbols.insert(symbol.to_string(), v);
                                }

                                symbols_displayed.insert(key, true);

                                let price_now = Decimal::from_str(event_price.as_str()).unwrap();
                                let to_send = (Symbol(v.clone()), price_now);

                                let volatility_count = *symbols_var_count
                                    .get(symbol.to_string().clone().as_str())
                                    .unwrap();

                                let percent = k.round_dp_with_strategy(2, RoundingStrategy::ToZero);

                                if volatility_count
                                    >= config.symbol_monitor.symbol_price_violatile_required_count
                                {
                                    // this symbol has required volatility - so we will send
                                    // it
                                    let msg = to_send.clone();
                                    info!(
                                    "{symbol}: sent to engine [price: {price_now}, diff: {percent}, volatility count: {volatility_count}]"
                                );
                                    channel_to_engine.send(msg).unwrap();

                                    symbols_sent.insert(symbol_type.clone(), true);
                                    let time = Instant::now();
                                    temp_monitored_symbols
                                        .insert(symbol.clone(), (time, price_now));
                                    biggest_monitored.insert(symbol.clone(), price_now);
                                }
                            }
                        }
                    }
                }
            }

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
