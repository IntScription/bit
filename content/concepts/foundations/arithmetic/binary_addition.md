# Binary Addition

Binary addition works exactly like the decimal addition you already
know — add column by column, right to left, and carry when a column
overflows. The only difference is *when* a column overflows: in
decimal a column overflows past 9, in binary it overflows past 1.

## The four rules

There are only four possible column additions in binary, and they're
worth memorizing outright:

```
0 + 0 = 0
0 + 1 = 1
1 + 0 = 1
1 + 1 = 10   (write 0, carry 1)
```

That last one is the entire trick. `1 + 1` can't be written as a
single binary digit — just like `9 + 1` can't be written as a single
decimal digit — so it becomes `0` in this column, with a `1` carried
into the next one.

## Example 1: no carries

```
  0101   (5)
+ 0010   (2)
------
  0111   (7)
```

Every column adds to 0 or 1, so nothing carries. Same as `5 + 2 = 7`.

## Example 2: one carry

```
  0011   (3)
+ 0001   (1)
------
  0100   (4)
```

Rightmost column: `1 + 1 = 0`, carry `1`. That carried `1` lands in
the next column: `1 + 0 + 0 = 1`. Result: `0100` = 4. Same as `3 + 1 = 4`.

## Example 3: a carry chain

```
   0111   (7)
+  0001   (1)
-------
   1000   (8)
```

Rightmost column: `1 + 1 = 0`, carry `1`. Next column: `1 + 0 + carry
1 = 0`, carry `1` again. Next column: `1 + 0 + carry 1 = 0`, carry `1`
again. Leftmost column: `0 + 0 + carry 1 = 1`. The carry rippled
through every column — this is exactly the same thing that happens in
decimal when you add `1` to `999` and get `1000`: every 9 flips to 0
and pushes a carry, until one finally sticks.

## Why this matters

This ripple-carry process is *literally* what a CPU's adder circuit
does in hardware — one small circuit per column, each one's carry-out
wired to the next column's carry-in. [[bit_shifting]] gives you
multiplication by powers of two for free; this gives you every other
arithmetic operation, since subtraction (add a [[twos_complement]]
negative), multiplication (repeated shifted addition), and even
checksums are all built on top of addition, not invented separately.

## Related

- [[binary]] — what you're adding: two binary numbers, same positional-value reading as always
- [[bit_shifting]] — multiplying by 2 is a shift; multiplying by anything else eventually comes back to repeated addition
- [[twos_complement]] — how subtraction reuses this same addition circuit instead of needing its own
