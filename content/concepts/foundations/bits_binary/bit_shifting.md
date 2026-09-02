# Bit Shifting

Take a binary value and slide every switch one position to the left. The
switches that fall off the end disappear; a fresh `0` switch fills in on
the right.

```
  0  0  0  0  0  1  0  1   = 5
shift left by 1 ->
  0  0  0  0  1  0  1  0   = 10
```

5 became 10 — shifting left by one place doubled it, the same way adding
a zero to the end of a decimal number multiplies it by ten. Shift right
and the opposite happens: the value halves (dropping any remainder).

```
  0  0  0  0  1  0  1  0   = 10
shift right by 1 ->
  0  0  0  0  0  1  0  1   = 5
```

Shifting is one of the cheapest operations a CPU can do — no real
"multiplication" happens, switches just slide over — which is why it's
used constantly for fast math and for packing several small values into
one larger one.

## Real world

- Fast multiplication/division by powers of two
- Packing multiple small values into a single integer
- Extracting a specific field out of binary data (combined with a mask)

## Related

- [[binary]] — what's actually sliding when you shift
- [[bytes]] — shifting is usually bounded by a fixed register width, like a byte
