To use Jolly, simply run the `jolly` executable. 

```bash
# Run Jolly with jolly.toml in the current directory
jolly
```

To use a config file that is not in the current directory, pass its path on the command line:

```bash
# Run Jolly with a custom config file
jolly /path/to/custom/jolly.toml
```

For more details on how Jolly finds its config file, see the
[documentation](file-format.md#locations).

By default, Jolly won't show any results: just tell you how many entries it has loaded:

![startup page](static/startup.png)

You can search for an entry by typing in text: Jolly will use the
title of the entry and any [tags](file-format.md#tags) associated
with the entry to find results:

![startup page](static/basic-search.png)

To open the entry, you can select it using the arrow and enter keys,
or click it with the mouse.

To learn more about the file format used by Jolly, see the [file-format](file-format.md) page.

To learn more about changing settings for Jolly, including how to customize the theme, see the [config](config.md) page.

To learn more advanced tips and tricks, see the [advanced](advanced.md) usage page.
