# Advanced examples: set, remove, add:*

## set/remove

The `set` and `remove` directives are meant to be used together with templating
in the modify scripts. For example, there might be a key binding in KDE you only
want on computers were a specific program is installed. This could be accomplished
by something like the following for `kglobalshortcutsrc`

```bash
{{if lookPath "my-fancy-program"}}
set "my-fancy-program.desktop" _k_friendly_name "My fancy program" separator="="
set "my-fancy-program.desktop" _launch "Ctrl+Shift+Y,none,my-fancy-program" separator="="
{{end}}

# Make sure the lines aren't added back into the config for all systems
# This should be outside the if statement
add:remove "my-fancy-program.desktop" _k_friendly_name
add:remove "my-fancy-program.desktop" _launch
```

(In this case, note that you might need to manage the `.desktop` file with
chezmoi as well. KDE normally creates these in `$HOME/.local/share/applications/`.)

Similarly, `remove` can be used to remove entries, but be careful when readding
the source files: If you blindly re-add the file on the computer where the lines
are filtered out, they will get lost for all computers.

## add:remove/add:hide

The directives `add:remove` and `add:hide` can be used to remove entries and
hide values respectively when re-adding files from the system to the chezmoi
source state.

Some use cases for this are:

* Use `add:hide` to prevent a password from being added back to the source state
  when you re-add a file with other changes. See the
  [konversationrc example](basics.md#konversationrc) for an example of this. By using
  `add:hide`, the line will still be present in the source file, but without its
  value. This ensures that the keyring transform is able to find it in the source
  state and do its work when checking out the file on a new system.
* Use `add:remove` to prevent a line from entering the source state at all. This
  can be useful together with system specific configuration with the `set`
  directive:
  ```bash
  {{ if (.is_work) }}
  set "Default Applications" "x-scheme-handler/jetbrains" "jetbrains-toolbox.desktop" separator="="
  {{ end }}
  # Completely remove the line when adding back (regardless of which computer this is on).
  add:remove "Default Applications" "x-scheme-handler/jetbrains"
  ```
  This example for the `mimeapps.list` file will add a specific line only if
  `is_work` is true. The `add:remove` directive helps prevent that line from being
  added back to the source state by mistake (where it would be applied to other
  computers unintentionally).

> **NOTE:** The `add:hide` and `add:remove` directives are processed as is
without going through chezmoi's template engine when re-adding files. This means
it won't matter if they are inside an if block, nor can you use template
expressions in their arguments.

> **NOTE:** `ignore` directives also result in an implicit `add:remove`. Again,
it doesn't matter if it is inside an if block or not currently during adding of
files, and any template expressions will not be expanded.

Both of these limitations *may* change in the future.
