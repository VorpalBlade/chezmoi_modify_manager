# Converting from chezmoi_modify_manager 1.x

Both the old and the new version have the same feature set. However the
rust implementation is about 50x faster (release builds, 25x in debug builds).

However the syntax is not compatible.

## Automatic conversion

There is a [script](../utils/conversion.sh) that can help if you have standard
shaped files (i.e. as created by the old `chezmoi_ini_add`), it will not handle
100% of the conversion for transforms however. The argument lists formats have
changed, as have some of the argument names.

Also, special consideration needs to be taken for the [keyring](#keyring)
transform.

## Manual conversion

The first line should now be used to invoke chezmoi_modify_manager. It should
be one of:

Use this if chezmoi_modify_manager is installed in PATH (recommended):
```
#!/usr/bin/env chezmoi_modify_manager
```

Use this if you keep chezmoi_modify_manager in your chezmoi source directory:
```
#!{{ .chezmoi.sourceDir }}/.utils/chezmoi_modify_manager-{{ .chezmoi.os }}-{{ .chezmoi.arch }}
```

In addition, the way to specify the source file has changed. The line to specify
the source file would now typically look like:

```
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
