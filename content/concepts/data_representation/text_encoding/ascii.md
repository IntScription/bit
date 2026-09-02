# ASCII

A byte is just a number. ASCII is nothing more than an agreement: "if a
byte holds this number, display this letter."

```
65  = A
97  = a
48  = 0  (the character '0', not the number 0)
32  = (space)
```

That's it — there's no deeper mechanism. `'A'` isn't stored specially;
the byte `01000001` genuinely is the number 65, and it's only the
agreement (the ASCII table) that says "when you're displaying text,
show an A."

ASCII only defines 128 characters — letters, digits, punctuation, a
handful of control codes — which fits in 7 bits, not all 8 of a byte.
That leftover bit is exactly what later encodings like [[unicode]] had
to figure out what to do with.

## Real world

- Plain text files
- The lowest 128 code points of Unicode, by design — `'A'` is 65 in both
- Legacy protocols and file formats still built around it

## Related

- [[bytes]] — what a character actually is underneath
- [[unicode]] — what happens once 128 characters isn't enough
