use crate::binance::state::BinanceState;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct BinanceObj {
    pub state: Arc<Mutex<BinanceState>>,
}
