# IP Addresses

An IPv4 address like `192.168.1.1` looks like a special new kind of
number. It isn't — it's exactly four [[bytes]], written as four
decimal numbers separated by dots. Nothing about it is new information
you don't already have.

```
192   .  168   .    1   .    1
11000000.10101000.00000001.00000001
```

Each of those four numbers is a single byte (0-255), read exactly the
way you already read [[binary]]: eight bits, positional value, done.
The dots are purely a human-readability convention — the address
itself is just 32 bits, the same kind of bit pattern as anything else
you've converted between binary and decimal so far.

That's also why this concept practices with the exact same puzzle as
Binary and Bytes: convert one octet (one of the four dot-separated
numbers) between its binary and decimal reading. An IP address puzzle
*is* a byte-conversion puzzle — it just comes with a networking label
on it.

## Real world

- Every device on the internet is reachable through one of these
- Subnet masks (AND-ing an address against a mask) decide which addresses are on the same local network
- Reading a router's config page or a `ping`/`traceroute` output

## Related

- [[bytes]] — an IP address is literally four of them
- [[binary]] — the same octet-by-octet conversion you already practice
- [[bitwise_masks]] — how a subnet mask actually filters which addresses match
