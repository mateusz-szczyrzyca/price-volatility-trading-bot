# Algorithm

This is a simple bot which monitors real time trades from all markets websocket
and builds up its own list which is called "windows".

This internal list consist of 3 parts: pre window, main window and post window. Further actions
depend on these windows, these can be set in `config.toml`. To understand what these mean follow
this document carefully.

## main window prices list - general rules

The bot creates it's on list of last trading prices with length `symbol_price_list_length` from `config.toml`.
This list is called "main window", Assuming we have `symbol_price_list_length=9` such list could look like as presented
below:

&nbsp;

`window=[1,2,3,4,5,6,7,8,9]`

&nbsp;

where `1` is the "oldest" price and `9` the newest. If a new transaction from
specific pair will happen, then oldest entry (on the left) is removed and new entry is inserted (on the right of the
list) to
keep the window list fixed length all the time.

&nbsp;

If we check only this main window, which means that `pre_window_analysis=false` and `post_window_analysis=false`
in `config.toml`
then in our example we can see that prices are rising from `1` in the beginning to `9` in the end.

Now bot checks config
values `post_window_price_value_rise_min_max_percent` and `post_window_price_value_drop_min_max_percent` which both are
list with
starting and ending threshold in percent and if this rise (or loss) reaches this configured threshold, then bot may try
to
start/finish trade or monitor (option `post_window_price_value_monitor_min_max_percent`) this pair.

## pre window and post window prices list

If options `pre_window_analysis=true` and/or `post_window_analysis=true` then `symbol_price_list_length` is divided into
not only main window but also pre window and/or post window.

If bot `pre_window` and `post_window` options are enabled, then this window `window=[1,2,3,4,5,6,7,8,9]` means: pre
window
"period" contains prices `[1,2,3]`, main window "period" contains `[4,5,6]` and post window period finally `[7,8,9]`.

If only `pre_window` is enabled then `[1,2,3,4,5]` is pre window part, and `[6,7,8,9]` is main window part.

If only `post window` is enabled then `[1,2,3,4,5]` is main window part, and `[6,7,8,9]` is post window part.

The general idea is: you can set this settings (this is just an example): if in some period specified pair price was
decreasing (pre window), and later it still was decreasing (main window) and finally wasn't increasing lately (post
window) then there is a chance that now it may increase rapidly so, based our config values, we should consider trading
this.
