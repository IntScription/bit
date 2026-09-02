# Linked Lists

An [[arrays|array]] is one contiguous block of memory — that's exactly
what makes `array[i]` a single address calculation, and exactly what
makes inserting something in the *middle* expensive (everything after
it has to shift over). A linked list solves that by giving up
contiguity entirely.

Instead of one block, a linked list is a chain of separate nodes, each
holding a value and a pointer to the *next* node:

```
head -> [12 | next] -> [45 | next] -> [7 | next] -> [88 | next] -> null
```

Inserting a new node anywhere just means rewiring two pointers — no
shifting anything. That's the entire tradeoff: arrays give you instant
`array[i]` access but expensive middle-insertion; linked lists give
you cheap insertion anywhere but no shortcuts to the middle — to reach
position 5, you have to walk the chain from the head, one `next` at a
time, through positions 0 through 4 first.

## Finding a value

Because there's no direct addressing, "what's at position 3" means
literally following the chain: start at the head (position 0), follow
`next` three times, read the value you land on. That step-by-step walk
is the entire puzzle mechanic for this concept — the same "find the
value at a position" question an array asks, just answered by walking
instead of jumping.

```
head -> [12] -> [45] -> [7] -> [88] -> [3] -> null
 pos:     0       1      2      3      4

Position 3? Walk: 12 -> 45 -> 7 -> 88. Value: 88.
```

## Real world

- Undo history and playlists, where items are added/removed from the middle constantly
- The building block behind a stack or queue implemented without a fixed-size backing array
- Browser history's back/forward chain

## Related

- [[arrays]] — instant indexing, at the cost of expensive middle-insertion
- [[stacks]] — often implemented as a linked list under the hood
- [[queues]] — same: a linked list handles growing/shrinking without a resize
