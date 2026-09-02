# Binary Division

Binary long division works exactly like decimal long division — for
each position, ask "does the divisor fit here?", write a 1 or 0
accordingly, and subtract if it does. Binary only ever asks yes/no,
never "how many times," which makes every step trivial.

## Worked example: 13 / 3

```
1101 (13) / 0011 (3)

Compare 3 against each prefix of 13's bits, left to right:
  1   -> 3 doesn't fit in 1          -> quotient bit 0
  11  -> 3 doesn't fit in 3? it does! -> quotient bit 1, remainder 0
  ...continue bringing down bits, comparing, subtracting...

quotient: 0100 (4), remainder: 1
```

`13 / 3 = 4` remainder `1` — check it against [[binary_multiplication]]:
`4 * 3 = 12`, plus the remainder `1`, gets back to `13`. Division and
multiplication undo each other, the same as in decimal.

This concept's puzzle asks for the **quotient only** (integer
division, remainder dropped) — the same thing `/` computes on unsigned
integers in most programming languages.

## The power-of-two shortcut

Just like multiplying by a power of two collapses into a left shift,
dividing by one collapses into a right shift:

```
1100 (12) / 0100 (4) = 1100 shifted right 2 places = 0011 (3)
```

No long division needed — this is the exact mirror image of
[[bit_shifting]]'s multiplication shortcut, and just as fast in
hardware.

## Why this matters

Outside the power-of-two shortcut, division is the slowest of the four
basic arithmetic operations on real hardware — it can't be reduced to
a handful of shifts and adds the way multiplication can, since each
digit of the quotient depends on a full compare-and-subtract against
what's left. That's a real, practical reason performance-sensitive
code goes out of its way to replace a division with a shift whenever
the divisor happens to be a power of two.

## Related

- [[binary_multiplication]] — the operation division reverses (quotient * divisor + remainder = dividend)
- [[binary_subtraction]] — what each long-division step actually performs
- [[bit_shifting]] — dividing by a power of two, with no long division required
