# Binary Search

If the array is already sorted, you don't need to check every slot —
you can eliminate half the remaining array with a single comparison.

```
[3, 7, 12, 19, 27, 45, 67, 93]   target: 45

check the middle (19): 45 > 19  -> target is in the right half
[27, 45, 67, 93]
check the middle (45... or 67 depending on rounding): compare, narrow again
...continue until found
```

Each comparison throws away half of what's left, so the number of
comparisons grows *logarithmically* with the array's size — doubling
the array only costs one extra comparison, not twice as many. That's
the entire reason binary search matters: it scales dramatically better
than [[linear_search]] on large arrays.

The catch, and it's a real one: binary search only works because the
array is sorted. Every "check the middle, discard a half" step depends
on knowing which half the target *must* be in, and that reasoning only
holds when the order is guaranteed. Search an unsorted array this way
and it can miss a target that's actually there.

## Real world

- Looking up a word in a sorted dictionary or index
- Database index lookups
- Finding the first version where a bug was introduced (bisecting)

## Related

- [[linear_search]] — what you fall back to when the array isn't sorted
- [[arrays]] — binary search needs direct index access, not just any sequence
