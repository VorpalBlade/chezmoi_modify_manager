# Converting from chezmoi_modify_manager 1.x

Both the old and the new version have the same feature set. However the
rust implementation is about 50x faster (release builds, 25x in debug builds).

However the syntax is not compatible.

Here is a guide to how to convert:

```bash

-ik key value -> ignore "key" "value"

-is section-name -> ignore section "section-name"

-ikr key-re value-re -> ignore regex "key-re" "value-re"

# Note change of argument order for transforms
-tk transform_name key value '{ "arg1": "value1", "arg2": "value2" }'
   -> transform "key" "value" transform_name arg1="value1" arg2="value2"

-tkr transform_name key value '{ "arg1": "value1", "arg2": "value2" }'
   -> transform regex "key" "value" transform_name arg1="value1" arg2="value2"
```

In addition, the way to specify the source file has changed. If you use the
recommended install into `PATH`, you can just put `source auto` into the file
if the source file follows standard naming conventions.

Otherwise, see the modify scripts generated when using `--add --style=template`.
