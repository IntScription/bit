# Hexadecimal

Binary is precise but long-winded: a single byte is 8 characters
(`10110110`). Hex fixes that by counting in base 16 instead of base 2 —
each hex digit stands in for exactly 4 bits (a *nibble*):

```
binary:  1011 0110
hex:       B    6
```

Sixteen values need sixteen digits, so hex borrows letters once it runs
out of numbers: `0 1 2 3 4 5 6 7 8 9 A B C D E F`. `A` is 10, `F` is 15.

`B6` in hex is `1011 0110` in binary is `182` in decimal — three ways of
writing the exact same byte. Hex wins on screen because a whole byte fits
in two characters instead of eight, which is why memory addresses, debug
output, and color codes are almost always shown in hex.

## Real world

- Memory addresses shown in debuggers
- Color codes — `#FF00AA` is three bytes of red, green, blue in hex
- Compact, human-friendly way to read raw byte values

## Related

- [[binary]] — what each hex digit is standing in for
- [[bytes]] — exactly two hex digits per byte
- [[decimal]] — the same value, a third way
