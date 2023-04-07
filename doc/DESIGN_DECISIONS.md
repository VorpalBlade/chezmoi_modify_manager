# Design descisions

This file documents motives some design descisions.

## Why did you implement a custom INI parser?

I ended up writing my own INI parser for rust:
[ini-roundtrip](https://github.com/VorpalBlade/ini-roundtrip). This had to
be done because standard INI parsers don't support preserving the
formatting. This is not acceptable when trying to minimise the diff. We
want to not change the formatting applied by the program that writes the
settings file. For example KDE writes `key=value` while PrusaSlicer writes
`key = value`.

It also does minimal parsing, meaning it can handle weird non-standard syntax
such as `[Colors:Header][Inactive]` (a real example from `kdeglobals`).

## Why Rust?

This code used to be written in Python, but each invocation of the command
would take on the order of 95 ms. Per managed file. As I was getting up to
around 20 managed INI files, this started to add up. The rewrite in Rust
takes (on the same computer) 2 ms. This is a 46x speedup. On another (faster)
computer I got a 63x speedup (54 ms vs 0.9 ms).
