use std::collections::HashMap;
use crate::binance::filters::FiltersParser;
use crate::config::settings::{ConfigStruct, CONFIG_FILENAME};
use crate::core::json::extract_json_data;
use crate::core::types::{Asset, BaseAsset, QuoteAsset, Symbol, SymbolAction};
use log::{error, info, warn};
use rand::{seq::IteratorRandom, thread_rng};
use rust_decimal::Decimal;
use std::fs;
use std::sync::{Arc, RwLock};

pub async fn update_symbols_and_filters_list(
    valid_trading_symbols: Arc<RwLock<HashMap<Symbol, bool>>>,
    symbol_actions: Arc<RwLock<HashMap<Symbol, SymbolAction>>>,
    filters_map: Arc<RwLock<HashMap<String, Decimal>>>,
) {
    let config_data = fs::read_to_string(CONFIG_FILENAME).expect("Cannot read config file {}");
    let config: ConfigStruct = toml::from_str(config_data.as_str()).unwrap();
    let api_exchange_info_addr = randomly_select_api_address(config.clone().exchange_info_apis);
    let json_string = fetch_exchange_info(api_exchange_info_addr.await)
        .await
        .expect("no json fetched");

    let _cfg = Arc::new(config.clone());

    let json_data = extract_json_data(&json_string).expect("cannot extract data from json string");

    let symbols_data = json_data["symbols"]
        .as_array()
        .expect("cannot create string to array (json symbols");

    ////////////////////////////////////////////////////////////////////////////////////////////
    // BEGIN: taking data about limits
    ////////////////////////////////////////////////////////////////////////////////////////////
    // info!("loading exchangeInfo data for processing...");

    'all_symbols_loop: for s in symbols_data.iter() {
        let mut this_is_spot_pair = false;

        //
        // BEGIN: "permissions": []
        //
        if s["permissions"].is_array() {
            let permission = s["permissions"].as_array().unwrap();

            for p in permission.iter() {
                if p.eq("SPOT") {
                    this_is_spot_pair = true;
                }
            }
        }

        // we want only spot market
        if !this_is_spot_pair {
            continue 'all_symbols_loop;
        }
        //
        // END: "permissions": []
        //

        // Rest symbols data
        let symbol: Symbol = Symbol(s["symbol"].to_string().replace(&['\"'][..], ""));
        let status: String = s["status"].to_string().replace(&['\"'][..], "");
        let b = s["baseAsset"].to_string().replace(&['\"'][..], "");
        let q = s["quoteAsset"].to_string().replace(&['\"'][..], "");

        let base_asset_val: BaseAsset = BaseAsset(b);
        let quote_asset_val: QuoteAsset = QuoteAsset(q);

        // excluding asset
        for asset in &config.excluded_assets {
            if base_asset_val.clone() == Asset::BaseAsset(BaseAsset(asset.clone())) {
                warn!("asset {asset} is excluded from the config.");
                continue 'all_symbols_loop;
            }
            if quote_asset_val.clone() == Asset::QuoteAsset(QuoteAsset(asset.clone())) {
                warn!("asset {asset} is excluded from the config.");
                continue 'all_symbols_loop;
            }
        }

        // excluding entire symbol and reversed symbol
        for sym in &config.excluded_symbols {
            if *sym == symbol {
                warn!("symbol {sym} is excluded by the config.");
                continue 'all_symbols_loop;
            }
        }

        // only symbols with main asset as base/quote are considered
        for my_base_asset in &config.base_starting_assets {
            let processed_asset = Asset::from(BaseAsset(my_base_asset.to_string()));
            if base_asset_val != processed_asset && quote_asset_val != processed_asset {
                // warn!("symbol: {symbol} does not have any base starting asset - skipping");
                continue 'all_symbols_loop;
            }

            // starting action for symbol
            if quote_asset_val == QuoteAsset(my_base_asset.to_string()) {
                symbol_actions
                    .write()
                    .unwrap()
                    .insert(symbol.clone(), SymbolAction::Buy);
            }
            if base_asset_val == BaseAsset(my_base_asset.to_string()) {
                symbol_actions
                    .write()
                    .unwrap()
                    .insert(symbol.clone(), SymbolAction::Sell);
            }
        }

        if status.eq("TRADING") {
            //
            // BEGIN: filters: [] - Support for filters
            //

            // price-filter-min
            // price-filter-max
            // price-filter-tick-size
            // pprice-multi-up
            // pprice-multi-down
            // pprice-avg-price-mins
            // lot-size-min-qty
            // lot-size-max-qty
            // lot-size-step-size
            // min-notional-min
            // min-notional-apply-to-market
            // min-notional-avg-price-mins
            // market-lot-size-min-qty
            // market-lot-size-max-qty
            // market-lot-size-step-size

            if s["filters"].is_array() {
                let filters = s["filters"].as_array().unwrap();

                let filter = FiltersParser::new(filters);

                {
                    // PRICE_FILTER: min_price
                    let key = format!("{}-price-filter-min", symbol.clone());
                    let value = filter.min_price;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // PRICE_FILTER: max_price
                    let key = format!("{}-price-filter-max", symbol.clone());
                    let value = filter.max_price;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // PRICE_FILTER: tick_size
                    let key = format!("{}-price-filter-tick-size", symbol.clone());
                    let value = filter.tick_size;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // PERCENT_PRICE: pprice-multi-up
                    let key = format!("{}-pprice-multi-up", symbol.clone());
                    let value = filter.percent_price_multi_up;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // PERCENT_PRICE: pprice-multi-down
                    let key = format!("{}-pprice-multi-down", symbol.clone());
                    let value = filter.percent_price_multi_down;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // PERCENT_PRICE: pprice-avg-price-mins
                    let key = format!("{}-pprice-avg-price-mins", symbol.clone());
                    let value = filter.percent_price_avg_min;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // LOT_SIZE: min-qty
                    let key = format!("{}-lot-size-min-qty", symbol.clone());
                    let value = filter.lot_min_qty;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // LOT_SIZE: max-qty
                    let key = format!("{}-lot-size-max-qty", symbol.clone());
                    let value = filter.lot_max_qty;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // LOT_SIZE: step-size
                    let key = format!("{}-lot-size-step-size", symbol.clone());
                    let value = filter.lot_step_size;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // MIN_NOTIONAL: minNotional
                    let key = format!("{}-min-notional-min", symbol.clone());
                    let value = filter.min_notional_min;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // MIN_NOTIONAL: applytomarket
                    let key = format!("{}-min-notional-apply-to-market", symbol.clone());
                    let value = filter.notional_apply_to_market;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // MIN_NOTIONAL: avgPriceMins
                    let key = format!("{}-min-notional-avg-price-mins", symbol.clone());
                    let value = filter.notional_avg_price_mins;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // MARKET_LOT_SIZE: min-qty
                    let key = format!("{}-market-lot-size-min-qty", symbol.clone());
                    let value = filter.market_lot_min_qty;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // MARKET_LOT_SIZE: max-qty
                    let key = format!("{}-market-lot-size-max-qty", symbol.clone());
                    let value = filter.market_lot_max_qty;

                    filters_map.write().unwrap().insert(key, value);
                }
                {
                    // MARKET_LOT_SIZE: step-size
                    let key = format!("{}-market-lot-size-step-size", symbol.clone());
                    let value = filter.market_lot_step_size;

                    filters_map.write().unwrap().insert(key, value);
                }
            }
            //
            // END: filters: []
            //

            // valid_trading_symbols contains only legitimate (on Binance) symbols and these will
            // be watched
            valid_trading_symbols
                .write()
                .unwrap()
                .insert(symbol.clone(), true);
        }
    }
    ////////////////////////////////////////////////////////////////////////////////////////////
    // END: Iteration over exchangeInfo data (taking symbols)
    ////////////////////////////////////////////////////////////////////////////////////////////

    info!(
        "exchangeInfo data processed, symbols found: {}",
        valid_trading_symbols.read().unwrap().len()
    );
}

pub async fn fetch_exchange_info(api: String) -> Option<String> {
    let exchange_info_url = api;

    let body = match reqwest::get(exchange_info_url).await {
        Ok(t) => t,
        Err(e) => {
            error!("request error: {}", e);
            return None;
        }
    };

    // request returned 429?
    let status_code = body.status();
    if status_code == 429 || status_code == 418 {
        // default value - 5 minutes
        warn!("status code 429 or 418 received: {status_code}");
        return None;
    }

    info!("exchangeInfo successfully fetched from API");

    Some(body.text().await.unwrap())
}

pub async fn randomly_select_api_address(list: Vec<String>) -> String {
    let mut rng = thread_rng();
    return list.iter().choose_stable(&mut rng).unwrap().to_string();
}
