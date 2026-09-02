# Selection Sort

Where [[bubble_sort]] compares *adjacent* pairs and swaps them as it
goes, selection sort asks a different question each pass: "what's the
smallest value in here, anywhere?" — then swaps just that one value
into place.

```
[5, 2, 8, 1]

Scan the whole array for the minimum: 1 (at index 3)
Swap it with whatever's at index 0: [1, 2, 8, 5]

End of pass 1: [1, 2, 8, 5] — index 0 is now correct, forever.
```

One pass, one swap (at most). The next pass repeats the same idea on
the remaining unsorted portion (`[2, 8, 5]`), finds *its* minimum, and
locks in index 1. And so on — each pass fixes exactly one more
position, from the front.

## Why this is a genuinely different shape than Bubble Sort

Bubble Sort's passes get *cheaper* as sorting progresses — later
passes have fewer adjacent pairs left to check, and a pass with zero
swaps means the array is already sorted, so it can stop early.
Selection Sort never gets that shortcut: finding "the minimum of
what's left" always means scanning every remaining element, every
single pass, even if the array happens to already be sorted. That's
the real tradeoff — Selection Sort is more predictable (always the
same amount of scanning) but can never finish early the way Bubble
Sort sometimes can.

## Real world

- A close cousin of Bubble Sort with a different (and more predictable) cost profile
- Repeatedly picking the next-best item from a remaining pool — the same shape as many selection/scheduling problems
- A useful contrast for understanding why some sorts have a best case and others don't

## Related

- [[bubble_sort]] — adjacent swaps instead of a full-array minimum scan
- [[arrays]] — what both sorts rearrange, one pass at a time
