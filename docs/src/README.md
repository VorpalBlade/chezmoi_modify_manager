# Introduction

`chezmoi_modify_manager` is an addon for [chezmoi](https://www.chezmoi.io/)
that deals with settings files that contain a mix of settings and state.
So far handling INI-style files are supported.

A typical example of this is KDE settings files. These contain (apart from
settings) state like recently opened files and positions of windows and dialog
boxes. Other programs (such as PrusaSlicer) also do the same thing.

`chezmoi_modify_manager` allows you to ignore certain sections of those
INI files when managing the configuration files with chezmoi.

## Features

* Ignore entire sections or specific keys in an INI style file.
* Ignore a key in a section based on regular expressions.
* Force set a value (useful together with templating).
* Force remove a section, key or entries matching a regex (useful together with templating).
* Apply a transformation to the value of a specified key. These are special
  operations that are built in and provide more complicated transformations.
  Some examples that this can do:
  * Look up a password in the platform keyring
  * Ignore the sorting order of a list style value (`key=a,b,c,d`)
  * etc.
* Assisted adding/updating of files in your chezmoi source state.
* *Optional* built in self-updater
