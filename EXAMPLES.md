# Examples - ignore and transform flags

Here are some useful examples of flags for various settings files I have come across.

## KDE

### dolphinrc
```bash
-is MainWindow
-is KPropertiesDialog
-ik General ViewPropsTimestamp
-ik "Open-with settings" History
```

### kdeglobals
```bash
-ik "KFileDialog Settings" "Show Inline Previews"
-is "DirSelect Dialog"
```

### kglobalshortcutsrc
There are two issues in this configuration.

First, ActivityManager switch-to-activity entries. There are multiple entries,
making it a perfect fit for a regular expression. Note that this is not state
per se. It does however seem to vary between computers, having different UUID
values.

Second, certain shortcut keys like flipping between two representations. A
specialised transform has been added to handle this case. When this is needed
you will see diffs like the following:

```diff
-playmedia=none,,Play media playback
+playmedia=none,none,Play media playback
```

In summary, the following seems to work well:

```bash
-ikr ActivityManager 'switch-to-activity-.*'
-tkr kde_shortcut ".*" ".*" "{}"
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
added to the chezmoi directory. Using an [add hook](#add-hook) can help with
this.

### kwinrc
Similar to kglobalshortcutsrc there are computer specific UUIDs.

```bash
-ikr Desktops 'Id_.*'
```

### plasmanotifyrc

```bash
-is DoNotDisturb
```

## PrusaSlicer / SuperSlicer

PrusaSlicer and the fork SuperSlicer also use INI style files:

### PrusaSlicer.ini / SuperSlicer.ini

```bash
-ik "<NO_SECTION>" auto_toolbar_size
-ik "<NO_SECTION>" freecad_path
-ik "<NO_SECTION>" last_output_path
-ik "<NO_SECTION>" last_output_path_removable
-ik "<NO_SECTION>" version_online
-ik "<NO_SECTION>" version_online_seen
-ik "<NO_SECTION>" version_system_info_sent
-ik "<NO_SECTION>" window_mainframe
-ikr "<NO_SECTION>" 'desktop_integration_.*'
-ik presets filament
-ik presets print
-ik presets sla_material
-ik presets sla_print
-ikr presets 'filament_.*'
-is recent
-is recent_projects
```

### PrusaSlicerGcodeViewer.ini / SuperSlicerGcodeViewer.ini

```bash
-is recent_projects
-ik "<NO_SECTION>" window_mainframe
```

## KeePassXC

## keepassxc.ini

KeePassXC stores private and public keys for KeeShare in the config.
You may not want to commit this to the repository.

```bash
-ik KeeShare Active
-ik KeeShare Foreign
-ik KeeShare Own
```

# Examples - hook scripts

## Add hook

A user defined hook script can optionally be executed by chezmoi_ini_add to
filter the data when adding it. This can be useful when readding files to
automatically remove passwords that are managed by a transform.

The hook script should be an executable file in the root of the chezmoi
directory and must be named `.chezmoi_modify_manager.add_hook`.

Here is an example that will filter out passwords of the `konversationrc` file:

```zsh
#!/bin/zsh
# The file from the target directory will be available on STDIN.
# The data to add to the source state should be printed to STDOUT.

# Currently only "ini"
type=$1
# Path of file as provided by the user to the command, may be a relative path
target_path=$2
# Path in the source state we are writing to. Will end in .src.ini for ini files.
source_data_path=$3

if [[ $source_data_path =~ konversationrc ]]; then
    # Filter out any set password.
    sed '/Password=./s/=.*$/=PLACEHOLDER/'
else
    # Let other files through as they are without changes
    cat
fi
```
