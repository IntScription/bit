# Binary Multiplication

Long multiplication in binary works the same way it does in decimal —
multiply by each digit of the second number, shift each partial result
into place, then add them all up. Binary just makes it simpler,
because every digit is 0 or 1: each partial product is either all
zeros or an exact (shifted) copy of the first number.

## Worked example

```
   0110   (6)
 * 0011   (3)
------
   0110    <- 6 * 1 (rightmost bit of 3), shift 0
  0110     <- 6 * 1 (next bit of 3), shift 1
------
  10010    (18)
```

Line up the two partial products (each one either `0000` or a shifted
copy of `0110`) and add them with plain [[binary_addition]]. `6 * 3 =
18`, and `10010` is 18 in binary.

## The power-of-two shortcut

If the second number is a power of two, every partial product except
one is all zeros — multiplication collapses into a single
[[bit_shifting|shift]]:

```
0110 (6) * 0100 (4) = 0110 shifted left 2 places = 011000 (24)
```

That's why `x * 4`, `x * 8`, `x * 16`, ... compile down to a single
shift instruction instead of a real multiply — there's no partial
products to add when only one bit in the multiplier is set.

## Why this matters

This "shift each partial product into place, then add" process is
exactly what a hardware multiplier circuit does — it's built entirely
out of shifters and adders, no separate "multiply" primitive
underneath. Every arithmetic operation past addition, all the way up
through multiplication, ultimately reduces to shifting and adding bits.

## Related

- [[binary_addition]] — what actually combines the shifted partial products
- [[bit_shifting]] — multiplying by a power of two, with zero addition required
- [[binary]] — the positional reading both operands share
