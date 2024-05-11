# Actions & directives

This is a high level overview of how chezmoi_modify_manager applies your config.
For the details on specific directives see `chezmoi_modify_manager --help-syntax`.

## Glossary

* Directive: Things like `source`, `ignore`, `transform`, `set`, `add:hide`, etc
  that you put in your config file. They are documented in the output of
  `chezmoi_modify_manager --help-syntax`.
* Actions: The directives are internally translated into a ruleset of actions.
  These are very similar to the directives, but may not correspond 1:1. For example:
  * `set` becomes a special transform internally.
  * `source` doesn't enter the actions, it is only used to figure out what file to load.
  * etc.

## Contexts

There are two different "contexts" for evaluating actions:

* Merging: This is the normal algorithm, used during `chezmoi apply` (and `diff` etc)
* Filtering: This is using when re-adding an existing file (`chezmoi_modify_manager -a`
  or `-s`).

See [Algorithms](algorithms.md) for details of how these work, in this file we 
are only concerned with how the directives and rules matching works.

These have separate directive to action translators. Not all directives apply to
all contexts. Some examples:

* `set` is unused when filtering
* `add:hide` is unused when merging
* `ignore` translates to the same as `add:remove` when filtering.
* etc.

## Order of action matching

Actions come in three flavours:

1. Section matches (always literal matches)
2. Literal section+key matches
3. Regular expression section+key matches

Not every rule can exist in every variant. For example:

* Merge section matches only support `ignore` and `remove`.
* `set` will only ever exist as a literal section+key match
* etc.

Chezmoi_modify_manager uses a single regex to match both the section and key.
This is done by constructing a combined string for these, using the 0-byte
(`\0`) as a separator. For example a regex directive
`ignore regex "Section|OtherSection" "SomePrefix.*"` is compiled down to
`(?:Section|OtherSection)\0(?:SomePrefix.*)`. This can be visible if you attempt
to use `^` or `$` (don't do that).

The special string `<NO_SECTION>` is used to match keys that appear before the
first section (hopefully no one has an ini-file with a section with that name in it).

When matching actions:

1. We first check if any section action applies. If so we are done. These are always literal matches.
2. Then we check if there is a literal section+key match. If so it applies and we are done.
3. Otherwise, we check if any regex action matches. If so we take the first result.
   This will be the same as first in source order in your config file.

Additionally, chezmoi_modify_manager will warn if there are multiple regex matches
that match. This can be disabled (per file) with a `no-warn-multiple-key-matches`
directive, in case you want this behaviour.
