# Examples

Here are some useful examples of flags for various settings files I have come across.

## KDE

### dolphinrc
```bash
-is MainWindow
-is KPropertiesDialog
-ik General ViewPropsTimestamp
```

### kdeglobals
```bash
-ik "KFileDialog Settings" "Show Inline Previews"
-is "DirSelect Dialog"
```

### kglobalshortcutsrc
Here a regular expression is more useful. This not state as such but a key
that seems to vary between different computers with different UUID values.

```bash
-ikr ActivityManager 'switch-to-activity-.*'
```

### konversationrc
Konversation has two relevant quirks:

1. It saves the password in the settings file (instead of using kwallet)
2. It resorts it alias list every time.

```bash
-ik ServerListDialog Size
-tk unsorted_list Aliases AliasList '{"separator": ","}'
-tk keyring 'Identity 0' Password '{"service": "konversation", "username": "konversation_id0"}'
```

To store the password for Identity 0 in your keyring of choice you can use the
`keyring` program installed by `python-keyring` (which is also required by
`chezmoi_ini_manager` for this functionality):

```console
$ keyring set konversation konversation_id0
[Enter your password at the prompt]
```

***Caution!*** Remember to also remove the password from the .src.ini that was
added to the chezmoi directory.

### kwinrc
Similar to kglobalshortcutsrc there are computer specific UUIDs.

```bash
-ikr Desktops 'Id_.*'
```

## PrusaSlicer / SuperSlicer

PrusaSlicer and the fork SuperSlicer also use INI style files:

### PrusaSlicer.ini / SuperSlicer.ini

```bash
-ik presets filament
-ik presets print
-is recent
-is recent_projects
-ik "<NO_SECTION>" last_output_path
-ik "<NO_SECTION>" last_output_path_removable
-ik "<NO_SECTION>" version_online
-ik "<NO_SECTION>" version_online_seen
-ik "<NO_SECTION>" version_system_info_sent
-ik "<NO_SECTION>" window_mainframe
-ikr "<NO_SECTION>" 'desktop_integration_.*'
```

### PrusaSlicerGcodeViewer.ini / SuperSlicerGcodeViewer.ini

```bash
-is recent_projects
-ik "<NO_SECTION>" window_mainframe
```
