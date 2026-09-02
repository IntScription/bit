# Unicode

[[ascii]] covers 128 characters — enough for English, barely. It has no
answer for 你好, emoji, é, ك, or the thousands of other characters
humans write with. Unicode is the fix: a single table assigning every
character in (almost) every writing system on Earth a unique number,
called a *code point*.

```
U+0041  = A          (same number as ASCII: 65)
U+4F60  = 你
U+1F600 = 😀
```

Notice `U+0041` is 65 — Unicode was deliberately designed so the first
128 code points match ASCII exactly. Anything valid ASCII is
automatically valid Unicode.

Unicode only defines *which number means which character* — it
deliberately says nothing about how those numbers get stored as bytes.
That's a separate problem, solved by encodings like [[utf8]].

## Real world

- Emoji, and every non-English writing system
- The character model virtually every modern text system is built on

## Related

- [[ascii]] — the 128-character baseline Unicode's first block matches exactly
- [[utf8]] — how a Unicode code point actually becomes bytes
