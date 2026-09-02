# Bit Rotation

[[bit_shifting]] moves bits left or right and lets whatever falls off
the end disappear, filling the empty end with zeros. **Rotation** does
almost the same thing, except nothing is ever lost — bits that fall
off one end wrap around and reappear at the other end.

```
10110000  rotate left 3
--------
10000101
```

Read it as: the leftmost 3 bits (`101`) fall off the left edge and get
stitched back on at the right edge, while everything else slides left
to make room. Rotating right is the mirror image — bits fall off the
right edge and reappear on the left.

Because nothing is discarded, rotation is reversible: rotating left by
`n` and then right by `n` always returns the original value. That
"nothing lost, fully reversible" property is exactly why rotation
shows up in cryptographic mixing functions, where you need to scramble
bits thoroughly without ever losing information.

## Real world

- Mixing steps inside cryptographic hash functions (e.g. SHA-family algorithms)
- Circular buffers and ring-shaped data layouts
- Hash table index scrambling

## Related

- [[bit_shifting]] — the non-wrapping cousin: bits that fall off are gone for good
- [[binary]] — rotation operates directly on the bit pattern
