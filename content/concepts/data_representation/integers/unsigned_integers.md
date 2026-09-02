# Unsigned Integers

You already know this one — it's every switch demo you've seen so far.
"Unsigned" just means: read the switches plainly, every bit counts
toward a positive total, nothing represents a sign.

```
128  64  32  16  8  4  2  1
```

With `n` switches you can represent `2^n` distinct values, always
starting at 0. Eight switches: 0 through 255. Sixteen switches: 0
through 65,535. There's no way to go negative — that's the entire idea
behind the name, and exactly the gap [[signed_integers]] exists to fill.

## Real world

- Array indices and sizes
- Memory addresses
- Counters that never go negative

## Related

- [[binary]] — the positional-value idea unsigned integers are built on
- [[signed_integers]] — what changes when negative numbers need representing
