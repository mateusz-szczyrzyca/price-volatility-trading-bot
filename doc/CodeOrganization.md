# Important parts of the code

If you prefer pictures, check `Architecture.pdf` first

### symbols_monitor.rs

Responsible for monitoring transactions on the exchange (via websocket) and tracking the last prices. If a specific
pairs matches our price of all non excluded symbols traded on SPOT market Windows thresholds then symbol monitor sends
such symbol via channel to `engine.rs`.

Algorithms checking threshold for pre_window, main_window and post_window are here. If these values matches our
thresholds then symbols monitor sends specific pair to `engine.rs` as potential candidate for trading.
But it does not mean this trade will happen - check more sections here.

&nbsp;

### engine.rs

Engine receives (via channel) chosen pair from symbols monitor and checks for basic trading possibilities:

- if we have a free slot - "free slot" is a free pool which we can use base currency for trading (USDT). For instance,
  assuming we have 200 USDT in our Wallet, and we set in `config.toml` such values
  as `max_simultaneously_trading_pairs=3`
  and `starting_asset_value=50` then we have `3` slots with `50 USDT` - in this case we can have only `3` concurrent
  tradings with budget `50 USDT`, so bot won't use entire `200 USDT` from the Wallet.

- some of pairs can't be traded too quickly if they were rejected or already finished (basically rejection and finished
  trading are the same terms here technically)

&nbsp;

### orderbook.rs

Main trading logic file - attaches to websocket for chosen pair and monitor it's orderbook entries and reacts
accordingly for profits, loses and timeouts.
