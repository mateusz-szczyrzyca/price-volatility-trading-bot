
## price-volatility-trading-bot

This is simple and easy to use trading bot written in Rust which supports (currently) Binance 
crypto exchange where it monitors trading actions and builds it's own internal in-memory database 
about short term trends amongst crypt pairs traded on the exchange. 

Using it's own database and configuration parameters, bot makes decision whether 
start to trade some pairs in hope for quick rise and profit. Bot tries to determine the most volatile pairs 
that it's the best chance to fast profit (or loss).

More details about the main algorithm and how to adjust it to your needs you will find here: doc/Algorithms.md

## WARNING

***This is a fully and old personal project without any guarantees - use solely at your own risk! Code may be not optimal. 
Depends on your configuration, bot may lose all of your deposit! For me it sometimes it was earning, sometimes it was loosing. 
*** 

Please read `config.toml` before you will try. 

By default, bot starts in simulation mode which only "simulates" real action. This mode is recommended for new users to study how 
bot trades. Simulation does not perform any real action on the exchange - it just inform what bot WOULD do if it would be in real trading mode. 

Keep in mind that some real action can/can't be executed on exchange for various reasons, including bugs in this software, hence 
simulation mode can't simulate 100% of cases as binance matching engine it's a 3rd party provider which acts here as a blackbox. 
Hence scenarios shown in simulation mode may/may not happen while real tradings.

There are some cases that are not supported yet, but are critical for your investment, the most notable: avoiding pairs which are 
about to be delisted soon. Bot sees big prices variations in such pairs and may trade them because of that. But when delisting 
happens you will end up with unusable pair in your wallet (on binance) - so keep in mind this particular scenario and in current 
version of the bot you have to check this delisting plans by yourself unless bot may "stuck" with useless pair.

Generally investing in cryptocurrencies is very risky and don't do it if you don't accept huge risk of loses.

In the config file you will find information how to run bot in real trading mode if you still interested in taking such risk.

# How to start?

1) Generate your API key on binance, API key should have as low permissions as possible, for security reasons.
   Use env vars to provide API keys: `BOT_API_KEY` and `BOT_SECRET_KEY`
2) Read `config.toml` - MANDATORY
3) Start bot in simulation mode and observe logs
4) Start bot in real trading mode - **DANGEROUS**


