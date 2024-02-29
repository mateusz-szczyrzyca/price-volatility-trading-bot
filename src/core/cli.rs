use crate::core::types::TradingMode;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, long_about = None)]
struct CliArgs {
    /// Perform real trading actions on the exchange (be careful!)
    #[arg(long)]
    real_trading_actions: bool,
}

pub fn determine_bot_trading_mode() -> TradingMode {
    let mut trading_mode = TradingMode::Simulation;

    let args = CliArgs::parse();

    if args.real_trading_actions {
        trading_mode = TradingMode::RealTrading
    }

    trading_mode
}
