# Stacks

A stack is an [[arrays|array]] with one rule bolted on: you can only
add or remove from a single end, called the **top**. That rule is
what gives a stack its behavior — **Last-In-First-Out (LIFO)**. The
most recent thing you added is always the first thing you get back.

```
push(5)  -> [5]
push(9)  -> [5, 9]
push(2)  -> [5, 9, 2]
pop()    -> returns 2, leaves [5, 9]
pop()    -> returns 9, leaves [5]
```

Two operations, that's the whole interface:

- **push(value)** — add a value on top
- **pop()** — remove and return the value currently on top

Picture a stack of plates: you can only take the top plate off, and
you can only add a new plate on top. You'd have to fully unstack it to
reach one near the bottom — which is exactly why "the last thing you
did is the first thing undone" makes a stack the natural fit for an
undo button, or for a program remembering which function to return to
next.

## Real world

- The undo button in almost every app
- Function call stacks during program execution
- Browser back-button history
- Balanced-parentheses / matching-bracket checking

## Related

- [[arrays]] — the structure underneath, with access restricted to one end
- [[queues]] — the FIFO counterpart: oldest in, first out instead of newest
