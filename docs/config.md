# Configuring Jolly
Configuration settings for Jolly are stored in the main `jolly.toml`
file.

They are stored in a special table called `[config]`. This means that
`config` cannot be used as a key for an Entry.

There are many settings in Jolly that are split into sub-tables. 

The subtables are described below in sections.

Please note that all of these settings are optional. If there are no
keys set, then Jolly will merely load the default configuration.

Jolly entries are separately described in [file-format.md](file-format.md)

Below is an example `config` section that could be in `jolly.toml`:

```toml
# set some settings for jolly

[config.ui]
width = 1000 # specify extra wide window
max_results = 7 # allow extra results

[config.ui.theme]
base = "dark"
accent_color= "orange"

[config.ui.search]
text_size = 40 # make search window bigger

[config.log]
file = 'path/to/logfile'
filters = "debug"

### Jolly entries below...

```


# [config.ui]
the `config.ui` table contains settings that control the appearance of
the Jolly window.

Below is more detail about the available settings: 


| field name    | data type | description                    |
|---------------|-----------|--------------------------------|
| `width`       | *integer* | width of Jolly Window          |
| `theme`       | *table*   | customize the theme of Jolly   |
| `search`      | *table*   | customize search field         |
| `results`     | *table*   | customize results display      |
| `entry`       | *table*   | customize result entries       |
| `text_size`   | *integer* | font size for UI.              |
| `max_results` | *integer* | max number of results to show. |
| `icon`        | *table*   | customize the display of icons |



## `width`        &mdash; *integer*

Determines the width of the Jolly window. Defined in virtual units as used by `iced`


## `search`       &mdash; *table*

This table contains additional settings. See below for details.

## `results`      &mdash; *table*

This table contains additional settings. See below for details.

## `entry`        &mdash; *table*

This table contains additional settings. See below for details.

## `text_size`        &mdash; *integer*

Specify the font size used for text in the Jolly UI. This is a special
setting parameter, because it also exists in the sub-tables `search`
and `entry`, in case you want to specify uniquely different text sizes for those UI elements

Default text size is 20. 

## `max_results`        &mdash; *integer*

Specify the maximum number of results to show in the Jolly search results window.

Defaults to 5 entries.


# [config.ui.theme]

These parameters control the theme of Jolly. Right now, theming
support is pretty basic and only supports setting the following parameters: 


| field name            | data type      | description                    |
|-----------------------|----------------|--------------------------------|
| `base`                | *string*       | base theme to use        |
| `accent_color`        | *color string* | color to use as main accent    |
| `background_color`    | *color string* | color to use for background    |
| `text_color`          | *color string* | color to use for text          |
| `selected_text_color` | *color string* | color to use for selected_text |


## `base`        &mdash; *'light'|'dark'*

Determine what base theme to use for Jolly UI. Currently, the only
options are 'dark' and 'light'.

If this variable is not set, Jolly will attempt to determine if the
current window manager is in a dark or light mode using
[dark-light](https://crates.io/crates/dark-light).

If `dark-light` is not successful, then the 'light' theme will be used
as a default.

If any of the other `config.ui.theme` parameters are set, they will
override the base values set by this variable. 

The default theme palette is described below:

### 'light' Theme
| field name            | value                |
|-----------------------|----------------------|
| `accent_color`        | *see below* |
| `background_color`    | 'white'              |
| `text_color`          | 'black'              |
| `selected_text_color` | 'white'              |

### 'dark' Theme
| field name            | value                |
|-----------------------|----------------------|
| `accent_color`        | *see below* |
| `background_color`    | '#202225             |
| `text_color`          | '#B3B3B3'            |
| `selected_text_color` | 'black               |



## `accent_color` &mdash; *color string*

Specify the accent color to use for the Jolly interface. 

This parameter is a string, but it is interpreted as an HTML color
using [csscolorparser](https://crates.io/crates/csscolorparser). This
means that [HTML named
colors](https://www.w3.org/TR/css-color-4/#named-colors) as well as
RGB values can be used to specify the accent color.

If the `accent_color` is left unspecified, then the behavior is platform specific: 

| Platform            | Behavior                                |
|---------------------|-----------------------------------------|
| Windows             | Uses `UIColorType::Accent` if available |
| All other Platforms | Uses default `iced` palette: '#5E7CE2'  |


## `background_color` &mdash; *color string*

Specify the background color to use for the Jolly interface. 

This parameter is a string, but it is interpreted as an HTML color
using [csscolorparser](https://crates.io/crates/csscolorparser). This
means that [HTML named
colors](https://www.w3.org/TR/css-color-4/#named-colors) as well as
RGB values can be used to specify the accent color.

If the `background_color` is left unspecified, then Jolly will use the
color specified by the `base` theme. 

## `text_color` &mdash; *color string*

Specify the text color to use for the Jolly interface. 

This parameter is a string, but it is interpreted as an HTML color
using [csscolorparser](https://crates.io/crates/csscolorparser). This
means that [HTML named
colors](https://www.w3.org/TR/css-color-4/#named-colors) as well as
RGB values can be used to specify the accent color.

If the `text_color` is left unspecified, then Jolly will use the
color specified by the `base` theme. 

## `selected_text_color` &mdash; *color string*

Specify the text color to use for the current selected Jolly Entry.

When using a custom `accent_color` value, it maybe necessary to tweak
this color to have enough contrast between the text and its
background.

This parameter is a string, but it is interpreted as an HTML color
using [csscolorparser](https://crates.io/crates/csscolorparser). This
means that [HTML named
colors](https://www.w3.org/TR/css-color-4/#named-colors) as well as
RGB values can be used to specify the accent color.

If the `text_color` is left unspecified, then Jolly will use the
color specified by the `base` theme. 

# [config.ui.search]

This table contains settings that control the search text window.

Currently it only has one setting: 

| field name     | data type      | description                 |
|----------------|----------------|-----------------------------|
| `text_size`    | *integer*      | font size for UI.           |


## `text_size`        &mdash; *integer*

Specify the font size used for text used by the search window. If the
key `config.ui.text_size` is already set, this key will override the
size only for the search text window.

Default text size is 20. 

# [config.ui.entry]

This table contains settings that control the entry results window

Currently it only has one setting: 

| field name     | data type      | description                 |
|----------------|----------------|-----------------------------|
| `text_size`    | *integer*      | font size for UI.           |


## `text_size`        &mdash; *integer*

Specify the font size used for text used by each entry result. If the
key `config.ui.text_size` is already set, this key will override the
size only for the entry results.

Default text size is 20. 

# [config.ui.icon]

*Only valid for Linux and BSD platforms*

This table contains settings for customizing how icons are displayed in Jolly.

Currently there is only one field available: 

| field name | data type | description                          |
|------------|-----------|--------------------------------------|
| `theme`    | *string*  | icon theme to use (Freedesktop only) |

## <a name="icon"></a> `theme` &mdash; *string*

The value of this option should be the name of a Freedesktop icon
theme to use on Linux and BSD platforms. There is no standard way to
specify which icon theme the user is using, so they should specify it
using this option. If this option is not set, then Jolly will use a
compile-time default, currently the `"gnome"` theme. If this theme is
not installed, then a fallback blank grey icon will be used for all
icons.

If you would like to change the compile time default theme, you can
use the environment variable `JOLLY_DEFAULT_THEME`.

For example, to build jolly with a default theme of "Adwaita": 

```
JOLLY_DEFAULT_THEME=Adwaita cargo build
```

As a general rule, the jolly build script will warn if the
`JOLLY_DEFAULT_THEME` doesn't seem to be installed at compile time.

# <a name="log"></a> [config.log]

The `[config.log]` table contains settings that control error logging
and debugging of Jolly. By default, Jolly will display error messages
in the UI and print them to `stderr`, but this behavior can be
customized.

**Important Note** *When you are trying to troubleshoot a bug in
Jolly, you may be asked to supply logfiles from operating
Jolly. Please be aware that at higher log levels, Jolly will include
the Jolly entry targets, and whichever text was entered in the search
window. This may be considered sensitive information and you should
always review logs before sharing them.*

To customize behavior, use the following fields:

| field name | data type                  | description                                                                                                                              |
|------------|----------------------------|------------------------------------------------------------------------------------------------------------------------------------------|
| `file`     | *string*                   | Additional filename to write logs to. Always appends.                                                                                    |
| `filters`  | *string* OR *string array* | [env_logger](https://docs.rs/env_logger/latest/env_logger/index.html#enabling-logging) filters for logging, by default, only log `error` |


As an example log file configuration, consider the following snippet: 

```toml
[config.log]

# Jolly logs are stored in log file below
file = 'path/to/logfile'

# messages from jolly crate at debug level, and cosmic_text crate at trace level
filters = ["jolly=debug", "cosmic_text=trace"]

```


## `file`        &mdash; *string*

Specify a filename for Jolly to write logs to. If the file cannot be
accessed then an error is returned. Jolly will always append to this
file if it already exists. Jolly will also continue to write trace
messages to `stderr`.

## `filters`        &mdash; *string* OR *string array*

The `filters` key can be used to specify one or more
[env_logger](https://docs.rs/env_logger/latest/env_logger/index.html#enabling-logging)
filters. These filters are used to determine which log level is
set. By default, only `errors` are logged, which also generally would
appear in the UI.

