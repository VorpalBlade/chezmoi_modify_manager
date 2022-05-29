# Design descisions

This file documents motives some design descisions.

## Why did you implement a custom INI parser?

The standard INI parser in Python's standard library reformats the file when
writing it back out. This is not acceptable when trying to minimise the diff.
We want to not change the formatting applied by the program that writes the
settings file. For example KDE writes `key=value` while PrusaSlicer writes
`key = value`.

It also does minimal parsing, meaning it can handle weird non-standard syntax
such as `[Colors:Header][Inactive]` (a real example from `kdeglobals`).

## Why Python 3.10?

While Python 3.10 is new as of writing this I wanted to try out using the
`match` statement. As I use Arch Linux I don't need support for older
versions.

## Why ZSH?

I use it, it is my preferred choice of shell. I did however make it
generate a bash script for the actual `modify_` scripts themselves for
maximum compatibility.
