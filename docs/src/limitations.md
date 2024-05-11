# Limitations

Alas, no software (apart from perhaps a simple `Hello, world!`) is perfect.
Here are some known limitations of `chezmoi_modify_manager`:

* When a key exists in the `.src.ini` file but not in the target state it will
  be added to the end of the relevant section. This is not an issue as the
  program will usually just resort the file next time it writes out its
  settings.
* `modify_` scripts bypass the check for "Did the file change in the target
  state" that chezmoi performs. This is essential for proper operation.
  However it also means that you will not be asked about overwriting changes.
  Always look at `chezmoi diff` first! I do have some ideas on how to mitigate
  this in the future. See also [this chezmoi bug](https://github.com/twpayne/chezmoi/issues/2244)
  for a more detailed discussion on this.
