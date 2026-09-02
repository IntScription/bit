# Checksums

Send a file across a network and something can go wrong along the way —
a bit flips, a packet gets corrupted. How does the receiver know?

The simplest possible answer: XOR every byte of the data together. The
single byte you get out is a *checksum*. Send the data and the checksum
together; if the receiver XORs the data again and gets a different
checksum than the one they were sent, something changed in transit.

```
bytes:     11001100  10101010  00001111
XOR all -> 11001100
         ^ 10101010
         ^ 00001111
         = 01101001   <- the checksum
```

This particular scheme is deliberately simple and deliberately weak —
it's the idea in its most transparent form, not what you'd actually
ship. Two flipped bits in the same position across two different bytes
cancel out and the checksum won't notice. Real systems use sturdier
math (CRC32, cryptographic hashes) for the same basic idea: compute
something small from the data, and recompute it to check nothing
changed.

## Real world

- Detecting corrupted downloads or transmissions
- Simple error detection in network protocols and file formats

## Related

- [[xor]] — the operation this entire scheme is built from
- [[bytes]] — the unit a checksum is computed over
