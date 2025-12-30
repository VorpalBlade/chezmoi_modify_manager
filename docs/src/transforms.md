# Transforms

This is a list of supported transforms. These are used to support some special
hard-to-handle cases. The general syntax is [documented elsewhere](configuration_files.md#transform),
but in short:

```bash
transform "section" "key" transform-name arg1="value" arg2="value" ...
transform regex "section-regex.*" "key-regex.*" transform-name arg1="value" ...
```

For example:

```bash
transform "mysection" "mykey" unsorted-list separator=","
```

Below is a list of supported transforms, but remember to check
`chezmoi_modify_manager --help-transforms` for the most up-to-date list.

## unsorted-list

Compare the value as an unsorted list.
Useful because Konversation likes to reorder lists.

Arguments:

* `separator=","`: Separating character between list elements

## kde-shortcut

Specialised transform to handle KDE changing certain global
shortcuts back and forth between formats like:

```ini
playmedia=none,,Play media playback
playmedia=none,none,Play media playback
```

No arguments.

## keyring

Get the value for a key from the system keyring. Useful for passwords
etc that you do not want in your dotfiles repo.

Arguments:

* `service="service-name"`: Service name to find entry in the keyring.
* `user="user-name"`: Username to find entry in the keyring.

You can add an entry to the secret store for your platform with:

```bash
chezmoi_modify_manager --keyring-set service-name user-name
```
