# XOR

XOR ("exclusive or") is OR with one condition removed: the output turns
on when the inputs **disagree**, and off when they agree.

```
0 XOR 0 = 0   (agree)
0 XOR 1 = 1   (disagree)
1 XOR 0 = 1   (disagree)
1 XOR 1 = 0   (agree)
```

Compare that last row to OR, where `1 OR 1 = 1`. That single flipped
value is the entire difference between OR and XOR.

XOR has a party trick regular AND/OR don't: applying it twice with the
same value undoes it. `X XOR K XOR K` always equals `X` again — which is
exactly why XOR shows up in bit-toggling and simple encryption.

## Real world

- Toggling a bit on and off with the exact same operation
- Simple stream ciphers (XOR a message with a key, XOR again with the same key to decrypt)
- Parity bits and basic error detection

## Related

- [[or]] — XOR is OR, minus the "both true" case
- [[and]] — the strict counterpart, requiring every input to agree
- [[boolean_logic]] — the true/false foundation all of these build on
