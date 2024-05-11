# Migration from version 1.x to 2.x

The new Rust code base has a superset of the features of the 1.x version, and it
is also about 50x faster (in release builds, about 25x in debug builds).

However, there is some work involved in migrating:
* Different [installation](#installation) method.
* The [syntax](#automatic-conversion-of-modify-scripts) has changed in the
  modify scripts.
* Some changes to [transforms](#transforms) (in particular the
  [keyring](#keyring) transform has changed).

In addition, the following differences are good to know about:
* The separate shell script to help with adding files is gone, the functionality
  is now built into the main program (see `--help` output).
* For binary installs from GitHub, you can now use a built in self updater
  (`--upgrade`).
* The regex syntax is different. Previously Python re module was used, now the
  [regex crate](https://docs.rs/regex/latest/regex/) for Rust is used. For most
  simple regular expressions there will be no relevant difference. However, some
  features (such as back references and look arounds) are not supported.
* Platform support with precompiled binaries is somewhat limited (compared to
  everything that Python supports). This is due to what I can build & test and
  what GitHub CI supports. Pull requests that enable testing and building for
  more platforms are welcome however (if the *tests cannot be executed* on
  GitHub CI, I will not accept it however).

## Installation

The methods of installation is different. No longer do you need (or should)
add this repo as a submodule in your dotfiles repo. Remove that and instead
see the [installation section](../README.md#installation) in the README.

## Modify scripts: Automatic conversion

There is a [script](../utils/conversion.sh) that can help if you have standard
shaped files (i.e. as created by the old `chezmoi_ini_add`).

It will not handle 100% of the conversion for transforms however. The argument
list format has changed, as have some of the argument names. See
[below](#transforms) for more details.

Also, special consideration needs to be taken for the [keyring](#keyring)
transform.

## Modify scripts: Manual conversion

The first line should now be used to invoke chezmoi_modify_manager. It should
be one of:

Use this if chezmoi_modify_manager is installed in PATH (recommended):
```bash
#!/usr/bin/env chezmoi_modify_manager
```

Use this if you keep chezmoi_modify_manager in your chezmoi source directory:
```bash
#!{{ .chezmoi.sourceDir }}/.utils/chezmoi_modify_manager-{{ .chezmoi.os }}-{{ .chezmoi.arch }}
```

In addition, the way to specify the source file has changed. The line to specify
the source file would now typically look like:

```bash
source "{{ .chezmoi.sourceDir }}/{{ .chezmoi.sourceFile | trimSuffix ".tmpl" | replace "modify_" "" }}.src.ini"
```

Finally, you need to convert the actual ignores and transforms themselves:

```bash

-ik key value -> ignore "key" "value"

-is section-name -> ignore section "section-name"

-ikr key-re value-re -> ignore regex "key-re" "value-re"

# Note change of argument order for transforms, the transform name
# now comes after the match.
-tk transform_name key value '{ "arg1": "value1", "arg2": "value2" }'
   -> transform "key" "value" transform-name arg1="value1" arg2="value2"

-tkr transform_name key value '{ "arg1": "value1", "arg2": "value2" }'
   -> transform regex "key" "value" transform-name arg1="value1" arg2="value2"
```

## Transforms

Transform arguments have changed. Before they were a JSON object, now they
are a series of `key="value"`.

Apart from that, transform names have changed:

* kde_shortcut -> kde-shortcut
* unsorted_list -> unsorted-list

Finally the argument name has changed for keyring: `username` is now just `user`.

## Keyring

As stated in the [previous section](#transforms), the argument names have
changed.

In addition, because the backend for talking to the platform secret store
is different, there can be other incompatibilities. Known ones include:

* On Linux, KDE KWallet is no longer supported. Only secret stores over
  DBus SecretService are supported. This means it will likely end up using
  GNOME's secret store (Seahorse) instead. See
  [the example for konversationrc](examples.md#konversationrc) for how to
  add the password, if you need to migrate.

Other platforms are untested (since I don't have any of those), but I
welcome any feedback to improve this documentation.
