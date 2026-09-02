# UTF-8

[[unicode]] hands you a code point — a number. UTF-8 is one way to turn
that number into actual bytes on disk, and it does it with a clever
constraint: **use as few bytes as you can get away with, and let each
byte announce how it should be read.**

```
'A'  (U+0041)   -> 1 byte:  01000001
'é'  (U+00E9)   -> 2 bytes: 11000011 10101001
'你'  (U+4F60)   -> 3 bytes: 11100100 10111101 10100000
'😀' (U+1F600)  -> 4 bytes: 11110000 10011111 10011000 10000000
```

Look at the leading bits of the *first* byte in each case: a plain `0`
means "this one byte is the whole character" — exactly an ASCII byte,
completely unchanged. A `110`, `1110`, or `11110` prefix announces "2,
3, or 4 bytes total," and every *continuation* byte after the first
starts with `10`. A program reading UTF-8 byte-by-byte always knows,
from the very first byte, exactly how many more to expect.

That first case is the important one: because ASCII bytes never start
with a `1`, **every valid ASCII file is already valid UTF-8, byte for
byte, unchanged.** That backward compatibility is a large part of why
UTF-8 won.

## Real world

- The dominant text encoding on the web and in most modern software
- Backward compatibility with plain ASCII files

## Related

- [[unicode]] — the code points UTF-8 is encoding
- [[ascii]] — why a plain-English file looks identical in both encodings
- [[bytes]] — the actual unit UTF-8 is packing code points into
