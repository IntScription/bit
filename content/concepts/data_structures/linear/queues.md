# Queues

A queue is an [[arrays|array]] with a different rule than a
[[stacks|stack]]: additions happen at one end, removals happen at the
other. That gives a queue **First-In-First-Out (FIFO)** behavior — the
first thing you added is always the first thing you get back.

```
enqueue(4)  -> [4]
enqueue(7)  -> [4, 7]
enqueue(1)  -> [4, 7, 1]
dequeue()   -> returns 4, leaves [7, 1]
dequeue()   -> returns 7, leaves [1]
```

Two operations, mirroring a stack's:

- **enqueue(value)** — add a value at the back
- **dequeue()** — remove and return the value at the front

Picture a line at a checkout counter: whoever got in line first gets
served first, no matter how many people join at the back afterward.
That fairness — first come, first served — is exactly why queues show
up anywhere order-of-arrival matters: print jobs, customer support
tickets, or a breadth-first search visiting the closest nodes before
the farther ones.

## Real world

- Print job and task scheduling
- Message queues in backend systems
- Breadth-first search traversal order
- Handling requests or customers in order of arrival

## Related

- [[arrays]] — the structure underneath, with access restricted to both ends
- [[stacks]] — the LIFO counterpart: newest in, first out instead of oldest
