# Linear Search

The simplest way to find something: start at index 0 and check every
slot in order until you find the target — or run out of array.

```
[19, 27, 67, 45, 93]   target: 45

index 0: 19 != 45
index 1: 27 != 45
index 2: 67 != 45
index 3: 45 == 45   -> found at index 3
```

If the target isn't there at all, linear search has to check every
single slot before it can say so — there's no shortcut. That's the
whole tradeoff: it never needs the array to be sorted, but it never
gets to be clever about where to look either. Every slot is checked
exactly once, in order, no assumptions.

That "no assumptions" property is exactly why it's the fallback:
[[binary_search]] is faster, but only works because it *assumes* the
array is sorted first. Linear search works on literally any array.

## Real world

- Scanning an unsorted list — there's no other option without sorting first
- Searching a small collection where sorting wouldn't pay off
- The fallback every other search strategy is judged against

## Related

- [[arrays]] — what linear search scans, one slot at a time
- [[binary_search]] — the faster alternative once the array is sorted
