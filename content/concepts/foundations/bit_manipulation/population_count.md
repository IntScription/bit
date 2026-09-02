# Population Count

Population count (sometimes called **popcount** or **Hamming weight**)
is just the number of `1` bits in a value. Nothing more exotic than
counting.

```
10110101
1  1 1 1 1  -> 5 bits set
```

It shows up constantly because "how many bits are on" turns out to
answer a lot of unrelated-sounding questions: how many permission
flags a user has, how far apart two binary values are (their **Hamming
distance** is the popcount of their [[xor]]), and whether a value has
even or odd parity — even parity means popcount is even, odd parity
means it's odd.

Real hardware often has a dedicated instruction for this exact
operation, because "count the 1 bits" is common enough in networking,
cryptography, and compression code to be worth a single CPU cycle
instead of a loop.

## Real world

- Parity and basic error-checking hardware
- Counting how many permission flags are active in a byte
- Hamming distance in coding theory and cryptography

## Related

- [[binary]] — popcount is a property of the bit pattern itself
- [[bitwise_masks]] — mask first to isolate the bits you want to count
- [[checksums]] — parity (even/odd popcount) is the simplest checksum there is
