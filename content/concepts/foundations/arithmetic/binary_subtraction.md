# Binary Subtraction

Binary subtraction works column by column, right to left, exactly like
decimal subtraction — including borrowing when a column runs out.

## The four rules

```
0 - 0 = 0
1 - 0 = 1
1 - 1 = 0
0 - 1 = 1   (borrow 1 from the next column)
```

That last one is the entire trick, same as decimal: you can't take 1
away from 0 in a single column, so you borrow a 1 from the column to
its left (which becomes a 2 in this column, in binary terms — the same
way borrowing in decimal turns a 0 into a 10).

## Example 1: no borrowing

```
  0111   (7)
- 0010   (2)
------
  0101   (5)
```

Every column subtracts cleanly. Same as `7 - 2 = 5`.

## Example 2: a borrow

```
  0100   (4)
- 0001   (1)
------
  0011   (3)
```

Rightmost column: `0 - 1`, borrow from the next column. That column
was `0`, so *it* borrows too, cascading left until it reaches the `1`
two columns over. This is the same ripple a decimal subtraction like
`400 - 1` goes through — borrowing has to travel until it finds
something to actually take from.

## When the result would go negative

Subtraction by hand assumes the top number is bigger. But what about
`0011 (3) - 0101 (5)`? The honest answer is `-2` — and representing
that negative result is exactly what [[twos_complement]] is for.

```
  3 - 5 = -2

-2 in 8-bit two's complement: flip the bits of 2 (00000010),
add 1 -> 11111110
```

So the puzzle for this concept sometimes hands you a subtraction where
the second number is bigger, and the expected answer is the 8-bit
two's complement bit pattern of the (negative) result — not a special
case bolted on, just the same negation process from Two's Complement
applied to whatever the subtraction actually comes out to.

## Why this matters

A CPU doesn't have a separate subtraction circuit. It computes `a - b`
as `a + (-b)`, where `-b` is `b`'s two's complement negation — the
exact same [[binary_addition]] circuit does both operations. That's
the entire reason two's complement exists as a representation in the
first place: it makes subtraction free once you already have addition.

## Related

- [[binary_addition]] — the circuit subtraction actually reuses
- [[twos_complement]] — how a negative result gets represented as bits
- [[binary]] — the positional reading both operands share
