# Two's Complement

Same eight switches as [[binary]]. Same positions. One change: the
leftmost switch now counts *against* the total instead of toward it.

```
-128  64  32  16  8  4  2  1
```

Toggle nothing: `00000000` = 0, same as always. Toggle only the sign
bit: `10000000` = -128, not +128. Toggle everything: `11111111` = -1.
That's the whole scheme — nothing else changes about how the switches
work, just what the leftmost one is worth.

## Negating a value

Flip every bit, then add one.

```
   5  =  00000101
flip  ->  11111010
 +1   ->  11111011  =  -5
```

Do it again and you're back to 5 — negation undoes itself, exactly like
you'd want. This trick is also *why* the scheme is called "two's
complement": flipping every bit is called the one's complement, and
adding one more gets you to two's.

## Why bother with something this specific?

Because it makes addition *free*. A CPU's adder circuit doesn't need a
special "subtract mode" or a separate code path for negative numbers —
`5 + (-3)` and `5 + 3` run through the exact same hardware, because -3
is just another 8-bit pattern that adds correctly. That's the entire
payoff: one adder, no special cases, which is why virtually every
mainstream CPU uses two's complement for signed integers.

## Real world

- How every mainstream CPU represents signed integers in hardware
- Integer overflow behavior in most programming languages

## Related

- [[signed_integers]] — the problem this scheme solves
- [[binary]] — the exact same switches, read differently
- [[xor]] — flipping every bit is XOR against all-ones
