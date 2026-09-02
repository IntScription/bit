# Bitwise Masks

A **mask** is a second byte you combine with your data, bit by bit, to
read, set, or clear specific bits without disturbing the rest — the
same three gates from [[boolean_logic]] ([[and]], [[or]], [[xor]]),
just applied across a whole register instead of one wire at a time.

```
  11010110   data
& 00001111   mask (keep the low nibble, clear the rest)
----------
  00000110
```

Three masking moves cover almost everything:

- **AND with a mask of 1s where you want to *keep* bits, 0s where you
  want to *clear* them.** Anything ANDed with 0 becomes 0; anything
  ANDed with 1 stays whatever it was.
- **OR with a mask of 1s where you want to *force on* bits.** Anything
  ORed with 1 becomes 1; anything ORed with 0 stays whatever it was.
- **XOR with a mask of 1s where you want to *flip* bits.** Anything
  XORed with 1 inverts; anything XORed with 0 stays whatever it was.

Notice the pattern: each gate has one mask value (0 for AND, 1 for OR,
0 for XOR) that leaves a bit untouched. That "untouched" value is what
makes masking useful — you can target exactly the bits you care about.

## Real world

- Extracting a color channel out of a packed RGB(A) pixel
- Setting, clearing, or checking permission flags in a single byte
- Reading specific status bits out of a hardware register

## Related

- [[and]] — the "keep or clear" mask operator
- [[or]] — the "force on" mask operator
- [[xor]] — the "flip" mask operator
- [[population_count]] — often the very next step after masking: count what you kept
