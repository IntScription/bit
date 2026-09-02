# Arrays

An array is a fixed number of same-sized slots, laid out back-to-back
in memory, each reachable by an index.

```
index:   0    1    2    3    4    5
value:  [12] [45] [ 7] [88] [ 3] [61]
```

The entire point of an array is that `array[i]` is a **direct address
calculation** — slot `i` lives at `start_address + i * slot_size` —
not a search. That's why reading or writing any index takes the same
amount of time no matter how big the array is, or which index you
pick. Compare that to a list you'd have to walk one item at a time to
find something in the middle: an array skips straight there.

That same fixed-slot layout is also an array's one real limitation:
its size is set up front. Adding a slot in the middle means shifting
every slot after it over by one, which is exactly the kind of thing
[[stacks]] and [[queues]] are built to avoid needing.

## Real world

- Storing a list of scores, pixels, or bytes contiguously in memory
- The backing storage behind strings, images, and buffers
- Lookup tables indexed directly by a code or id

## Related

- [[bytes]] — each slot in a byte array is exactly one byte
- [[stacks]] — an array with access restricted to just one end
- [[queues]] — an array with access restricted to both ends
