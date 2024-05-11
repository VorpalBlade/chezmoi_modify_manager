# Troubleshooting

The first step should be to run `chezmoi_modify_manager --doctor` and correct
any issues reported. This will help identify some common issues:

* chezmoi_modify_manager needs to be in `PATH`
* `**/*.src.ini` needs to be ignored in the root `.chezmoiignore` file
* Old chezmoi and/or using `CHEZMOI_MODIFY_MANAGER_ASSUME_CHEZMOI_VERSION`, see
  [this documentation](doc/source_specification.md) for more details on when or
  when not to use this.
