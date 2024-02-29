use serde::*;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceSymbol {
    pub base_asset: String,
    pub base_asset_precision: i64,
    pub contract_type: String,
    pub delivery_date: i64,
    pub filters: Vec<Filter>,
    pub liquidation_fee: String,
    pub maint_margin_percent: String,
    pub margin_asset: String,
    pub market_take_bound: String,
    pub onboard_date: i64,
    pub order_types: Vec<String>,
    pub pair: String,
    pub price_precision: i64,
    pub quantity_precision: i64,
    pub quote_asset: String,
    pub quote_precision: i64,
    pub required_margin_percent: String,
    pub settle_plan: i64,
    pub status: String,
    pub symbol: String,
    pub time_in_force: Vec<String>,
    pub trigger_protect: String,
    pub underlying_sub_type: Vec<String>,
    pub underlying_type: String,
    pub permissions: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub filter_type: String,
    pub max_price: Option<String>,
    pub min_price: Option<String>,
    pub tick_size: Option<String>,
    pub max_qty: Option<String>,
    pub min_qty: Option<String>,
    pub step_size: Option<String>,
    pub limit: Option<i64>,
    pub notional: Option<String>,
    pub multiplier_decimal: Option<String>,
    pub multiplier_down: Option<String>,
    pub multiplier_up: Option<String>,
}
