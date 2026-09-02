# AND

Two switches feed into an AND gate. The output only turns on if **both**
inputs are on.

```
0 AND 0 = 0
0 AND 1 = 0
1 AND 0 = 0
1 AND 1 = 1
```

One way to remember it: AND is the strictest operator. Every single
input has to agree before the output does.

## Real world

- Permission checks that require every condition to be satisfied ("must be logged in AND an admin")
- Bitmasking to clear specific bits (AND with 0 forces a bit off)
- Hardware AND gates, physically etched into a CPU

## Related

- [[boolean_logic]] — what true/false means before combining them
- [[or]] — the "any input agrees" counterpart
- [[not]] — inverts a single input instead of combining two
