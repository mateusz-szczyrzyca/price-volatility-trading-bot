use crate::binance::exchange_info::update_symbols_and_filters_list;
use crate::binance::object::BinanceObj;
use crate::binance::symbols_monitor::all_trades_websocket;
use crate::core::engine::engine;
use crate::core::types::Symbol;
use log::info;
use rust_decimal::Decimal;
use std::sync::Arc;
use std::thread;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time;

impl BinanceObj {
    pub async fn start(self: Arc<Self>) {
        // thread: listen for prices and update them

        // Channel from symbol monitor ->
        info!("starting exchange object");
        #[allow(clippy::type_complexity)]
        let (symbol_monitor_sender, symbol_monitor_receiver): (
            UnboundedSender<(Symbol, Decimal)>,
            UnboundedReceiver<(Symbol, Decimal)>,
        ) = mpsc::unbounded_channel();

        info!("fetching exchangeInfo for the first time...");
        // exchangeInfo first run to receive data
        let s = self.clone();
        tokio::spawn(async move {
            // comment
            let valid_symbols_map = s.state.lock().unwrap().valid_trading_symbols.clone();
            let filters_map = s.state.lock().unwrap().filters_map.clone();
            let symbol_actions = s.state.lock().unwrap().default_symbol_action.clone();
            update_symbols_and_filters_list(valid_symbols_map, symbol_actions, filters_map).await;
        })
        .await
        .expect("cannot retrieve exchangeInfo");
        info!("exchangeInfo fetched for the first time...");

        tokio::time::sleep(time::Duration::from_secs(2)).await;
        //
        // THREAD 1: perodically fetches exchangeInfo and updates symbols list and filters for them
        //           These data are exposed outside by list_symbols() and process_price_and_qty()
        //

        info!("starting symbol monitor collector ...");
        {
            let s = self.clone();
            let config = s.state.lock().unwrap().config.clone();
            let filters_map = s.state.lock().unwrap().filters_map.clone();
            let symbol_actions = s.state.lock().unwrap().default_symbol_action.clone();
            let api_key = s.state.lock().unwrap().api_key.clone();
            let secret_key = s.state.lock().unwrap().secret_key.clone();
            let trading_mode = s.state.lock().unwrap().trading_mode.clone();
            tokio::spawn(async move {
                // comment
                engine(
                    config,
                    filters_map,
                    symbol_actions,
                    symbol_monitor_receiver,
                    (api_key, secret_key),
                    trading_mode,
                )
                .await;
            });
        }
        info!("candlestick monitor collector started");

        info!("starting monitor thread...");
        {
            let s = self.clone();
            let config = s.state.lock().unwrap().config.clone();
            let valid_symbols_map = s.state.lock().unwrap().valid_trading_symbols.clone();
            thread::spawn(move || {
                // comment
                all_trades_websocket(config, valid_symbols_map, symbol_monitor_sender);
            });
        }
        info!("monitor thread started");

        // exchangeInfo
        let s = self.clone();

        let handle_exchange_info = tokio::spawn(async move {
            // comment
            loop {
                let config = s.state.lock().unwrap().config.clone();
                let valid_symbols_map = s.state.lock().unwrap().valid_trading_symbols.clone();
                let filters_map = s.state.lock().unwrap().filters_map.clone();
                let symbol_actions = s.state.lock().unwrap().default_symbol_action.clone();
                update_symbols_and_filters_list(valid_symbols_map, symbol_actions, filters_map)
                    .await;
                time::sleep(time::Duration::from_secs(
                    config.exchange_info_fetch_delay_secs,
                ))
                .await;
            }
        });

        // standard
        let handles = vec![handle_exchange_info];

        for handle in handles {
            handle.await.unwrap();
        }

        info!("Exchange processor started.");
    }
}
