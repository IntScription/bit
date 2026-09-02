# Signed Integers

[[unsigned_integers]] can't go negative — every switch pattern maps to a
positive number, full stop. But a thermostat needs to show -5°, a bank
balance needs to go below zero, a function needs a way to signal
"error" distinct from every valid positive result.

A signed integer is the same fixed set of switches, but with a scheme
that reserves some of that range to mean "negative." The most obvious
approach — set aside one switch as a plain +/- flag — turns out to have
awkward edge cases (two different patterns both meaning zero, and
addition needing special-case logic). [[twos_complement]] is the scheme
virtually every modern computer actually uses instead, and it avoids
both problems.

## Real world

- Temperatures, balances, coordinates — anything that can go negative
- Function return codes, where negative often signals an error

## Related

- [[unsigned_integers]] — the positive-only baseline this extends
- [[twos_complement]] — the actual encoding scheme in use today
