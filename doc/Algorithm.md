# Algorithm

WARNING: not finished yet

This is a simple bot which monitors real time trades from all markets websocket 
and builds up its own list which is similar to these lists

This internal list consist of 3 parts: pre window, window and post window. Further actions 
depend on these windows.

## Example

For explanation, we may consider three pairs: `AB`, `BC` and `CD`. There are many
more that can be traded on the exchange, but we care only about those.

We assume entire window length = 15, which means `pre_window`, `window` and 
`post_window` contains last 5 prices for tokens as `pre_window`+`window`+`post_window` is 
the entire monitoring window

and last best bid prices for these tokens are as follows:

```
A = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]
B = [5,2,3,5,4,6,7,8,9,10,11,12,13,14,10]
C = [10,2,7,3,7,1,8,2,7,3,4,1,3,3,3,1,3]
```

Depends on our configuration, we can perform action on exchange if:

1) `pre_window` and `window` prices dropped withing our limit and `post_window` price is about stable. 

This does not mean later price will be dropping/staying same/rising - it just our conditions when we 
consider "the good moment" to join to the trade this specific pair by bot. Keep in mind the market is 
unpredictable, cryptomarket especially. If this algorithm will work for two pairs, it does not mean it 
will work for another 2 pairs.