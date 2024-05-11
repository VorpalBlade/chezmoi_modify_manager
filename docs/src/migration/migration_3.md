# Migration from version 2.x to 3.x

## Migrating from hook scripts

In 2023 hook scripts were deprecated and then removed in early 2024 in version 3.0. They
are now replaced by the `add:remove` and `add:hide` directives.

For example, to make sure that a password isn't added back into the source
state you might use something like this:

```bash
transform "LoginSection" "Password" keyring service="myprogram" user="myuser"
# Make sure the password isn't added back into the config file on re-add
add:hide "LoginSection" "Password"
```

This would pull the password from the OS keyring, but erase it when doing
a re-add.

The `add:remove` directive can be used to completely remove the entry instead.
This can be useful together with `set` and system specific configuration:

```bash
{{ if (.is_work) }}
set "Default Applications" "x-scheme-handler/jetbrains" "jetbrains-toolbox.desktop" separator="="
{{ end }}
# Completely remove the line when adding back (regardless of which computer this is on).
add:remove "Default Applications" "x-scheme-handler/jetbrains"
```

This example for mimeapps.list would add an entry only when .is_work is true,
but also make sure that the value isn't added back to the config file and thus
prevents transferring it to other computers by mistake.

> **NOTE:** The `add:hide` and `add:remove` directives are processed as is
without going through chezmoi's template engine when re-adding files. This means
it won't matter if they are inside an if block, nor can you use template
expressions in their arguments.

> **NOTE:** `ignore` directives also result in an implicit `add:remove`. Again,
it doesn't matter if it is inside an if block or not currently during adding of
files.
