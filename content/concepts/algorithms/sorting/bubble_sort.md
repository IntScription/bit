# Bubble Sort

One full pass: walk the array left to right, and for every adjacent
pair, swap them if the left one is bigger. That's it. That's the
entire rule.

```
[5, 2, 8, 1]

compare (5, 2): out of order -> swap -> [2, 5, 8, 1]
compare (5, 8): in order     -> no swap -> [2, 5, 8, 1]
compare (8, 1): out of order -> swap -> [2, 5, 1, 8]

end of pass: [2, 5, 1, 8]
```

Notice what happened to the 8: it kept winning every comparison it was
part of, so it "bubbled" all the way to the end of the array in a
single pass — that's where the name comes from. In fact, this is
guaranteed: after one full pass, the largest value is always in its
final, correct position. Run a second pass over the remaining
unsorted part and the second-largest value locks into place too, and
so on.

That guarantee is also bubble sort's honest weakness: locking in one
value per pass means a fully unsorted array of length n can take up to
n-1 passes to finish. It's easy to trace by hand, which is exactly why
it's the first sorting algorithm most people learn — but it's not what
production code reaches for on a large array.

## Real world

- The simplest sorting algorithm to hand-trace or teach
- Detecting whether a list is already sorted (zero swaps in a pass)
- A baseline every faster sorting algorithm is compared against

## Related

- [[arrays]] — what bubble sort rearranges, one swap at a time
- [[linear_search]] — another "check everything, one at a time" algorithm
