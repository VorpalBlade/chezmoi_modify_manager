# Algorithms

This documents a high level overview of the algorithms chezmoi_modify_manager
uses for merging or filtering INI files.

In general these algorithms are single-pass, processing one line at a time
from the input INI file. This makes them quite fast in practice.

The code for these are implemented in the [ini-merge] crate. The explanation
here is intended for users, and as such leaves out a lot of gnarly details around
for example: empty sections, sections with only comments in them, etc. If you are
interested in that, go read the code.

The actual INI parser is in the [ini-roundtrip] crate. A custom INI parser is used
to ensure that writing it back out doesn't change formatting.

# Filtering

This is used when re-adding an existing file (`chezmoi_modify_manager -a` or `-s`).
This is the simpler of the two algorithms.

Relevant directives from your config for this algorithm:
* `add:hide` (replaces the value with `HIDDEN` when re-adding)
* `add:remove` (removes the line when re-adding)
* `ignore` (removes the line when re-adding)

(The reason there are two directives with the same effect is that they do
different things when merging.)

When a user passes `-s` or `-a` a bunch of things happen:

1. We figure out if the file is already managed or not. Depending on `-a` or `-s`
   we will then do different things. This is not the focus of this page though.
2. Assuming we decided that we should add the file and manage it using
   `chezmoi_modify_manager` (instead of plain `chezmoi`), and that the file was
   *already* managed by us before, we then need to filter:
   1. Load the ruleset that the user wrote into an [Actions structure](actions.md).
      Currently, this does not take chezmoi templates into account (though this
      might change).
   2. For each line in the file being filtered:
      * If it is a new section header, check [section actions](actions.md) to
        determine if it should be removed entirely, otherwise keep it.
      * If it is a comment or blank line keep it (unless the entire section is
        being removed)
      * If it is a key, check [actions](actions.md) to determine if it should be
        hidden, removed or kept.

Note evaluation order of actions documented in [Actions](actions.md#order-of-action-matching),
section matches take priority, then literal matches, then regex matches (in order).

# Merging

This is used for normal `chezmoi apply` (and `chezmoi diff` etc). This is a more
complicated case: there are now three files involved.

Relevant directives from your config for this algorithm:
* `ignore` (keeps the system state and ignores whatever is in the source state)
* `set` (sets to a specific key and value)
* `remove` (entirely removes the match)
* `transform` (applies a custom transform to the match, see `--help-transforms`,
  custom semantics apply to each)

1. Load the ruleset that the user wrote into an [Actions structure](actions.md).
   Chezmoi has already processed any templates for us.
2. Load the `.src.ini` file into a fast data structure for looking things up in it.
3. For each line in the system state (as provided by chezmoi on stdin):
   * If it is a comment or blank line, keep it (unless it is in a section
     that we are not outputting).
   * If it is a section header, check:
      * If the entire section is ignored, keep it as is from the system state.
      * If the section is being removed by `remove`, remove it.
      * If the section exists in the .src.ini, keep it.
      * If the section *doesn't* exist in the .src.ini, remove it.
      * (There is also some additional logic to deal with entirely empty
        sections etc, so we don't actually emit the section on stdout until we
        are sure later on, there is a concept of "pending lines" to implement that.)
   *  If it is a key, find the first [action that applies](actions.md#order-of-action-matching)
      if any. Then:
      * If no action applies, take the value from the `.src.ini` file.
      * If no action applies and the line is not in the `.src.ini` file, remove
        the line.
      * If the action is to `ignore`, leave the system value as is.
      * If the action is to `remove`, remove it.
      * If the action is to `set`, set it.
      * If a transform applies, apply it (see each transform for more details).
   * Before we start a new section, check if there are any lines in the
     `.src.ini` that didn't exist in the system state (or any such `set`
     directives), if so emit them.
   * Before the end of the file, check for entire sections (or `set` directives
     in such sections) in the `.src.ini` that didn't exist in the system state,
     if so emit them.

The newly emitted keys or sections from the last two bullet points will
generally be weirdly formatted. The assumption is the program that owns this
file will reformat it on next use.

[ini-merge]: https://github.com/VorpalBlade/ini-merge
[ini-roundtrip]: https://github.com/VorpalBlade/ini-roundtrip
