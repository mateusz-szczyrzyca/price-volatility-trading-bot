use std::collections::HashMap;
use crate::binance::orderbook::orderbook_executor;
use crate::config::settings::{ConfigStruct, CONFIG_FILENAME};
use crate::core::calc::percent_diff;
use crate::core::structs::OrderBookCommand;
use crate::core::trading::TradingSymbol;
use crate::core::types::{KlineSignal, OrderBookCmd, Symbol, SymbolAction, TradingMode};
use log::{info, warn};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use std::fs;
use std::ops::Not;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task;
use tokio::time::Instant;

/*
Takes symbols from symbol monitor and starts threads with trading tasks
 */

pub async fn engine(
    config: ConfigStruct,
    filters_map: Arc<RwLock<HashMap<String, Decimal>>>,
    symbol_actions: Arc<RwLock<HashMap<Symbol, SymbolAction>>>,
    mut channel_from_monitor: UnboundedReceiver<(Symbol, Decimal)>,
    api_keys: (String, String),
    trading_mode: TradingMode,
) {
    info!("engine started");

    if trading_mode == TradingMode::Simulation {
        warn!("!!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !! !!! !!! !!!");
        warn!("!!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !! !!! !!! !!!");
        warn!("!!! !!! THIS IS SIMULATION MODE -  NO REAL TRADES WILL TAKE PLACE  !!! !!!");
        warn!("!!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !! !!! !!! !!!");
        warn!("!!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !!! !! !!! !!! !!!");
    }

    let mut symbols_already_processing: HashMap<Symbol, bool> = HashMap::new();
    let tasks: Arc<Mutex<HashMap<Symbol, Sender<KlineSignal>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let mut currently_trading_pairs = 0;
    let mut currently_trading_reminder_time = Instant::now();
    let mut symbols_traded_recently: HashMap<Symbol, Instant> = HashMap::new();
    let mut previous_cmd_read_time = Instant::now();
    let mut stop_accepting_symbols = false;

    // channel
    #[allow(clippy::type_complexity)]
    let (executor_signal_out, executor_signal_receiver): (
        Sender<TradingSymbol>,
        Receiver<TradingSymbol>,
    ) = mpsc::channel();

    let mut driving_channels_map: HashMap<Symbol, Sender<OrderBookCommand>> = HashMap::new();

    let base_qty_pool = config.starting_asset_value;
    let mut profits_list: Vec<Decimal> = Vec::new();
    let mut available_pools_list: Vec<Decimal> = Vec::new();

    let cmd_instant_sell_file =
        format!("{}/{}", config.cmd_dir, config.cmd_stop_and_sell_instantly);

    for _n in 0..config.max_simultaneously_trading_pairs.to_i32().unwrap() {
        let pool = base_qty_pool;
        info!("creating new asset pool with value={pool}...");
        available_pools_list.push(pool);
    }

    let initial_pool_value: Decimal = available_pools_list.clone().iter().sum();
    let initial_pool_length = Decimal::from(available_pools_list.len());
    let decimal_zero = Decimal::ZERO;
    let mut c = config;
    loop {
        if previous_cmd_read_time.elapsed().as_secs() >= c.clone().cmd_read_period_secs {
            previous_cmd_read_time = Instant::now();
            // read cmd for instant sell
            if Path::new(&cmd_instant_sell_file).exists() {
                //
                for (sym, channel) in driving_channels_map.iter() {
                    info!("sending request for instant sell to {sym} orderbook executor...");

                    let cmd = OrderBookCommand {
                        cmd: OrderBookCmd::StopAndInstantSell,
                    };

                    channel.send(cmd).unwrap();
                }

                stop_accepting_symbols = true;

                let res = fs::remove_file(&cmd_instant_sell_file);
                if res.is_err() {
                    println!("problem with removing file: {cmd_instant_sell_file}");
                }
            }
        }

        if currently_trading_reminder_time.elapsed().as_secs()
            >= c.clone()
                .orderbook_monitor
                .currently_trading_reminder_period_secs
        {
            let config_data =
                fs::read_to_string(CONFIG_FILENAME).expect("Cannot read config file {}");
            let config_new: ConfigStruct = toml::from_str(config_data.as_str()).unwrap();
            c = config_new;

            let map_copy = symbols_already_processing.clone();
            let keys = map_copy.keys();
            let list_to_sum = available_pools_list.clone();
            let list_sum: Decimal = list_to_sum.iter().sum();
            info!("---");
            info!("STATUS: currently trading {currently_trading_pairs} pairs => {keys:?}");
            info!(
                "STATUS: currently available base pools: {:?}",
                available_pools_list.clone()
            );
            //
            // for (_, v) in driving_channels_map.iter() {
            //     let cmd = OrderBookCommand {
            //         cmd: OrderBookCmd::StopAndInstantSell,
            //     };
            //
            //     v.send(cmd).unwrap();
            // }

            if Decimal::from(available_pools_list.len()) == initial_pool_length {
                //
                if initial_pool_value > decimal_zero {
                    let profit_percent = percent_diff(initial_pool_value, list_sum)
                        .round_dp_with_strategy(2, RoundingStrategy::ToZero);
                    info!("STATUS: sum of currently available base pools: {list_sum} / {initial_pool_value} [profit: {profit_percent}%]");
                }
            }

            if c.clone().orderbook_monitor.use_profits_to_trade.not() {
                let profits: Decimal = profits_list.iter().sum();
                info!(
                    "STATUS: profits so far (use_profits_to_trade=false): ===> {profits} USDT <==="
                );
            }
            info!("---");
            currently_trading_reminder_time = Instant::now();
        }
        let cfg = c.clone();
        //
        // BEGIN: symbols finished trading
        //
        if let Ok(msg) = executor_signal_receiver.try_recv() {
            let trading_symbol = msg;

            let symbol = trading_symbol.symbol;
            let received_qty = trading_symbol.qty;
            let used_qty = trading_symbol.used_qty;
            let sum_qty =
                (received_qty - used_qty).round_dp_with_strategy(2, RoundingStrategy::ToZero);

            info!(
                "[from orderbook executor]: finished trading: {}, qty: {}, used: {}, profit: {}",
                symbol.clone(),
                received_qty,
                used_qty,
                sum_qty
            );

            // received_qty == decimal_zero means LIMIT SELL ORDER is left - no profit now, but
            // make pool free
            if received_qty > decimal_zero && used_qty > decimal_zero {
                // can be negative
                let profit =
                    (received_qty - used_qty).round_dp_with_strategy(2, RoundingStrategy::ToZero);
                profits_list.push(profit);
            }

            // symbol is returned so get back to the pool
            available_pools_list.push(trading_symbol.started_qty);

            let task_map = Arc::clone(&tasks);
            let mut map = task_map.lock().unwrap();
            map.remove(&symbol.clone());

            symbols_traded_recently.insert(symbol.clone(), Instant::now());

            symbols_already_processing.remove(&symbol);
            driving_channels_map.remove(&symbol);
            currently_trading_pairs -= 1;
        }

        //
        // BEGIN: RECEIVING FROM CHANNEL: symbol to trade
        //
        if let Ok(msg) = channel_from_monitor.try_recv() {
            // symbol received
            let price = msg.1;
            let symbol_string = msg.0.to_string();
            let symbol = msg.0;

            let mut symbol_is_allowed_to_trade_now = true;

            if symbols_already_processing.contains_key(&symbol.clone()) || stop_accepting_symbols {
                // symbol is already processing so we can't process it again
                symbol_is_allowed_to_trade_now = false;
            }

            if symbols_traded_recently.contains_key(&symbol.clone())
                && symbol_is_allowed_to_trade_now
            {
                let traded_time_ago_sec = *symbols_traded_recently.get(&symbol.clone()).unwrap();

                if traded_time_ago_sec.elapsed().as_secs()
                    < c.clone()
                        .orderbook_monitor
                        .break_between_trading_same_symbol_secs
                {
                    symbol_is_allowed_to_trade_now = false;
                    warn!("{symbol} REJECTED: delay between past and next trading for this symbol is still in force.")
                } else {
                    // remove
                    symbols_traded_recently.remove(&symbol.clone());
                }
            }

            if available_pools_list.is_empty() {
                warn!(
                    "currently trading {}/{} pairs, so {} is REJECTED for now",
                    currently_trading_pairs, cfg.max_simultaneously_trading_pairs, symbol_string
                );
                symbol_is_allowed_to_trade_now = false;
            }

            if symbol_is_allowed_to_trade_now {
                info!("[from symbol_monitor]: symbol: {symbol}, price: {price} - TRADING");
                let task_map = Arc::clone(&tasks);

                // channel: => executor
                let (signal_sender, _signal_receiver): (
                    Sender<KlineSignal>,
                    Receiver<KlineSignal>,
                ) = mpsc::channel();

                let map_filters;
                let map_symbols;
                {
                    map_filters = filters_map.read().unwrap().clone();
                    map_symbols = symbol_actions.read().unwrap().clone();
                }
                let s = symbol.clone();
                let trading_mode = trading_mode.clone();
                let executor_ch = executor_signal_out.clone();

                // take something from pool and remove
                let trading_symbol_qty_pool = available_pools_list.remove(0);
                let api_keys_tuple = api_keys.clone();

                // channel for orderbook
                #[allow(clippy::type_complexity)]
                let (orderbook_sender, orderbook_receiver): (
                    Sender<OrderBookCommand>,
                    Receiver<OrderBookCommand>,
                ) = mpsc::channel();

                driving_channels_map.insert(s.clone(), orderbook_sender);

                task::spawn_blocking(move || {
                    orderbook_executor(
                        cfg.clone(),
                        s,
                        map_filters,
                        map_symbols,
                        trading_symbol_qty_pool,
                        price,
                        orderbook_receiver,
                        executor_ch,
                        api_keys_tuple,
                        trading_mode,
                    );
                });

                // workaround for tokio::spawn to spawn previous task always (!)
                task::spawn(async move {});

                task_map
                    .lock()
                    .unwrap()
                    .insert(symbol.clone(), signal_sender);

                // increment list of trading pairs
                currently_trading_pairs += 1;
                symbols_already_processing.insert(symbol.clone(), true);
            }
        }
        //
        // END: RECEIVING FROM CHANNEL: symbol to trade
        //
    }
}
