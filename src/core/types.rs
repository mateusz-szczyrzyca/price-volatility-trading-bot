use serde::Deserialize;
use std::fmt;
use strum_macros::EnumString;

#[derive(Deserialize, Debug, Clone, Eq, Hash)]
pub struct Symbol(pub String);

unsafe impl Sync for Symbol {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum TradingMode {
    Simulation,
    RealTrading,
}

#[derive(Default, Debug, Clone, Hash)]
pub struct EmptyAsset(pub String);

#[derive(Default, Debug, Clone, Eq, Hash)]
pub struct BaseAsset(pub String);

#[derive(Default, Debug, Clone, Eq, Hash)]
pub struct QuoteAsset(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum Asset {
    BaseAsset(BaseAsset),
    QuoteAsset(QuoteAsset),
    EmptyAsset,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum KlineSignal {
    Start,
    SoftStop,
    KillNow,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum SymbolAction {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum TradingNextStep {
    Join,
    Leave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString)]
pub enum TradingDecision {
    Start,
    Continue,
    Decline,
    Stop,
    Wait,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum CurrentTradingProfit {
    Unknown,
    AbsoluteMinimalProfit,
    MinimalProfit,
    GoodProfit,
    Loss,
    LossTooLarge
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum ReadMarketDepthNow {
    YES,
    NO
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum Trading {
    Start,
    Decline,
    Hold,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum OrderBookCmd {
    StopAndInstantSell,
    StopAndLimitSell,
}

impl From<bool> for SymbolAction {
    fn from(value: bool) -> Self {
        if value {
            SymbolAction::Buy
        } else {
            SymbolAction::Sell
        }
    }
}

impl From<String> for Symbol {
    fn from(value: String) -> Self {
        Symbol(value)
    }
}

impl From<BaseAsset> for Asset {
    fn from(value: BaseAsset) -> Self {
        Asset::BaseAsset(value)
    }
}

impl From<QuoteAsset> for Asset {
    fn from(value: QuoteAsset) -> Self {
        Asset::QuoteAsset(value)
    }
}

impl From<QuoteAsset> for BaseAsset {
    fn from(value: QuoteAsset) -> Self {
        BaseAsset(value.to_string())
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for BaseAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for QuoteAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Asset::BaseAsset(value) => write!(f, "{}", value.0),
            Asset::QuoteAsset(value) => write!(f, "{}", value.0),
            Asset::EmptyAsset => write!(f, ""),
        }
    }
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl PartialEq for BaseAsset {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl PartialEq for QuoteAsset {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl PartialEq<BaseAsset> for QuoteAsset {
    fn eq(&self, other: &BaseAsset) -> bool {
        self.to_string() == other.to_string()
    }
}

impl PartialEq<QuoteAsset> for BaseAsset {
    fn eq(&self, other: &QuoteAsset) -> bool {
        self.to_string() == other.to_string()
    }
}

impl PartialEq<Asset> for QuoteAsset {
    fn eq(&self, other: &Asset) -> bool {
        self.to_string() == other.to_string()
    }
}

impl PartialEq<Asset> for BaseAsset {
    fn eq(&self, other: &Asset) -> bool {
        self.to_string() == other.to_string()
    }
}
