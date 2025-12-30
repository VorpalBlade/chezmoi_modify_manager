# Syntax of configuration files

chezmoi_modify_manager uses basic configuration files to control how to
merge INI files. These are the `modify_<config_file_name>` files. They can
also be templated with `chezmoi` by naming the file
`modify_<config_file_name>.tmpl` instead. The easiest way to get started is
to use `-a` to add a file and generate a skeleton configuration file.

## Syntax

The file consists of directives, one per line. Comments are supported by
prefixing a line with #. Comments are only supported at the start of lines.

> **Note!** If a key appears before the first section, use `<NO_SECTION>` as the
section.

> **Note!** The modify script can itself be a chezmoi template (if it ends with
`.tmpl`), which can be useful if you want to do host specific configuration using
the `set` directive for example.\
\
This however will slow things down every so slightly as chezmoi has to run its
templating engine on the file. Typically, this will be an overhead of about half
a millisecond per templated modify script (measured on an AMD Ryzen 5 5600X).

## Directives

### source

This directive is required. It specifies where to find the source file
(i.e. the file in the dotfile repo). It should have the following format
to support Chezmoi versions older than v2.46.1:

```bash
source "{{ .chezmoi.sourceDir }}/{{ .chezmoi.sourceFile | trimSuffix ".tmpl" | replace "modify_" "" }}.src.ini"
```

From Chezmoi v2.46.1 and forward the following also works instead:

```bash
source auto
```

### ignore

Ignore a certain line, always taking it from the target file (i.e. file in
your home directory), instead of the source state. The following variants
are supported:

```bash
ignore section "my-section"
ignore section regex "^MySection.*"
ignore "my-section" "my-key"
ignore regex "section.*regex" "key regex.*"
```

* The first form ignores a whole section (exact literal match).
* The second form ignores a whole section (regex match).
* The third form ignores a specific key (exact literal match).
* The fourth form uses a regex to ignore a specific key.

Prefer the exact literal match variants where possible, they will be
marginally faster.

An additional effect is that lines that are missing in the source state
will not be deleted if they are ignored.

Finally, ignored lines will not be added back when using `--add` or
`--smart-add`, in order to reduce git diffs.

### set

Set an entry to a specific value. This is primarily useful together with
chezmoi templates, allowing you to override a specific value for only some
of your computers. The following variants are supported:

```bash
set "section" "key" "value"
set "section" "key" "value" separator="="
```

By default, separator is `" = "`, which might not match what the program that
the ini files belongs to uses.

Notes:

* Only exact literal matches are supported.
* It works better if the line exists in the source & target state, otherwise
  it is likely the line will get formatted weirdly (which will often be
  changed by the program the INI file belongs to).

### remove

Unconditionally remove everything matching the directive. This is primarily
useful together with chezmoi templates, allowing you to remove a specific
key or section for only some of your computers. The following variants are
supported:

```bash
remove section "my-section"
remove "my-section" "my-key"
remove regex "section.*regex" "key regex.*"
```

(Matching works identically to ignore, see above for more details.)

### transform

Some specific situations need more complicated merging that a simple
ignore. For those situations you can use transforms. Supported variants
are:

```bash
transform "section" "key" transform-name arg1="value" arg2="value" ...
transform regex "section-regex.*" "key-regex.*" transform-name arg1="value" ...
```

(Matching works identically to ignore except matching entire sections is
not supported. See above for more details.)

For example, to treat `mykey` in `mysection` as an unsorted comma separated
list, you could use:

```bash
transform "mysection" "mykey" unsorted-list separator=","
```

The full list of supported transforms, and how to use them can be listed
using `--help-transforms`.

### add:remove & add:hide

These two directives control the behaviour when using --add or --smart-add.
In particular, these allow filtering lines that will be added back to the
source state.

`add:remove` will remove the matching lines entirely. The following forms are
supported:

```bash
add:remove section "section name"
add:remove "section name" "key"
add:remove regex  "section-regex.*" "key-regex.*"
```

(Matching works identically to ignore, see above for more details.)

`add:hide` will instead keep the entries but replace the value associated with
those keys. This is useful together with the keyring transform in particular,
as the key needs to exist in the source or target state for it to trigger
the replacement. The following forms are supported:

```bash
add:hide section "section name"
add:hide "section name" "key"
add:hide regex  "section-regex.*" "key-regex.*"
```

(Matching works identically to ignore, see above for more details.)

### no-warn-multiple-key-matches

This directive quietens warnings on multiple regular expressions matching the
same section+key. While the warning is generally useful, sometimes you might
actually "know what you are doing" and want to suppress it.
