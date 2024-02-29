use crate::core::types::OrderBookCmd;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderBookCommand {
    pub cmd: OrderBookCmd,
}
