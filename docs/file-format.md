# Jolly File Format


Jolly expects there to be a single file that contains database
entries, named `jolly.toml`. This file must have entries encoded using
the [TOML](https://toml.io) markup language.

You can find out more details about the syntax of TOML on the above
webpage, but the basics are defined below. 

For the purposes of this example, we will refer to an example `jolly.toml` file located in this documentation (you can find a full version of the file [here](jolly.toml)):

```toml
# Example Jolly File
['Edit Jolly Configuration']
location = 'jolly.toml'
description = "use your system's default program to open this jolly database"

['Jolly Quick Start Guide']
location = 'https://github.com/apgoetz/jolly/tree/main/docs'
desc = "Open the Jolly manual in your web browser"

['(W)ikipedia: %s']
url = 'https://en.wikipedia.org/w/index.php?title=Special:Search&search=%s'
keyword = 'w'
desc = "Search Wikipedia"

['Open Calculator']
system = 'calc.exe'
tags = ['math', 'work']
desc = """Open the calculator app. This example uses Window's
calc.exe, but for other OS's you may need change this entry.

For example, on Debian, you can use 'gnome-calculator' and on MacOs you can use
'/System/Applications/Calculator.app/Contents/MacOS/Calculator'
"""

['Send email to important recipient']
location = 'mailto:noreply@example.com'
tags = ['email']
desc = """Jolly entries don't just have to be web urls. 
Any protocol handler supported by your OS can be an entry. """
```

## Jolly Entries


A jolly entry at its most basic is composed of a *name* and an *entry
target*. The *name* represents a userfriendly title of the entry,
whereas the *entry target* represents the description of how jolly can access that
entry.

Each entry can also have an optional [description](#desc) that provides more detailed information about the entry.

Each entry can also have an optional [icon](#icon) field, which allows overriding the icon image to use for that entry. 

Jolly treats each table in the TOML file as its own entry, and the key of the table is treated as its *name*. 

The *entry target* of an entry is specified using a special key in the TOML table. The various types of *entry targets* are described below. 

For example, in the following entry: 

```toml
['Edit Jolly Configuration']
location = 'jolly.toml'
```

The *name* of the entry would be "Edit Jolly Configuration". This is
the text that would be displayed to the user if the entry is selected.

The *entry target* in this case would be of type `location`, and points to a local file called `jolly.toml`. 


**Important Note** *It is best practice to surround the entry *name*
in single quotes in your toml file. This is because if your entry name
contains a dot (.) the TOML parser will interpret the entry as a
hierarchical table, which will mess up Jolly's parsing*

**Important Note** The Settings of Jolly are specified in a special
table called `config` and are described in [config.md](config.md)



## <a name="tags"></a> Tags


In order to help with finding entries in a large `jolly.toml` file, Jolly supports using *tags* to provide more description about an entry. 

Tags are specified in a [TOML array](https://toml.io/en/v1.0.0#array) in the table entry. 

For example: 

```toml
['Open Calculator']
system = 'calc.exe'
tags = ['math', 'work']
```

In this entry, the *tags* are `math` and `work`. The user could search
for this entry using any of the phrases 'open', 'math', 'work', and
see the entry selected.

## <a name="desc"></a> Description

The *description* field can be used to provide an additional
description of the Entry. This field is optional, but if it is
present, the description text will be displyed under the entry in the
results.

This field is also aliased as *desc*, so either of the following will work: 

```toml
['Edit Jolly Configuration']
description = "Use your system's default program to open this Jolly database"
location = 'jolly.toml'
```

```toml
['Edit Jolly Configuration']
desc = "Use your system's default program to open this Jolly database"
location = 'jolly.toml'
```

*Description* fields can be multiple lines. In this case, line breaks
are skipped unless there are two newlines. (Same behavior as markdown
paragraphs). If you try to use any other markdown syntax, Jolly will
render your description as a unformatted text block (with hard newlines). 

Lastly, *description* fields do not support using `%s` as a keyword
parameter, unlike the title field. 

## <a name="icon"></a> Icon

Jolly entries are displayed with an icon image next to them. The icon
that is displayed is queried with OS-specific APIs, and should be the
same icon that is displayed in the platform's file explorer. If you
would like to override the icon that is chosen to be displayed, you
can use the `icon` field to specify a path to an image to use for that
icon. Jolly uses the [image](https://crates.io/crates/image) crate to
load images, which means that only image formats supported by that
crate can be used as icons with jolly. Additionally, Jolly is built
with SVG support (handled separately). This means that the image type
must be one of the following:

| Support Image Type | Recognized File Extensions |
|--------------------|----------------------------|
| PNG                | .png                       |
| JPEG               | .jpg, .jpeg                |
| GIF                | .gif                       |
| WEBP               | .webp                      |
| Netpbm             | .pbm, .pam, .ppm, .pgm     |
| TIFF               | .tiff, .tif                |
| TGA                | .tga                       |
| DDS                | .dds                       |
| Bitmap             | .bmp                       |
| Icon               | .ico                       |
| HDR                | .hdr                       |
| OpenEXR            | .exr                       |
| Farbfeld           | .ff                        |
| QOI                | .qoi                       |
| SVG                | .svg                       |


## Jolly Entry Target Types


Jolly supports the following types of *entry targets*. to specify an
*entry target* type, create a key with the corresponding name in the
entry table.

+ `location`
+ `system`
+ `keyword` entries
+ `url` 

### `location` Entry


A `location` entry is the most common type of entry for use with Jolly. The target of a location entry will be opened up using your systems default opening program. 

Importantly, this target does not just need to be a file, it can be any URL or URI that your system understands. (This is known as a *protocol handler*)

For example, to refer to an important PDF document, located in a deep directory:

```toml
[My very important file]
location = 'C:\Very\Deep\And\Hard\To\Remember\Directory\Document.PDF'
```

Or you could have a location that points to a website:

```toml
['Jolly Homepage']
location = 'https://github.com/apgoetz/jolly'
tags = ['docs','jolly']
```

Or an entry that pops up your mail program to compose a message to a commonly used email address: 

```toml
['Complain to Jolly Developers']
location = 'mailto:example@example.com'
```

The location entry is what makes Jolly powerful, because it inherits every type of protocol handler that your operating system understands. 

Many applications will register their own protocol handlers, which means that you will be able to use these links in Jolly to trigger those applications. For example, Microsoft defines a list of protocol handlers for Office products [here](https://learn.microsoft.com/en-us/office/client-developer/office-uri-schemes). 

To learn more about protocol handlers on different operating systems, please see the following documentation: 

+ [Windows](https://www.howto-connect.com/choose-default-apps-by-protocol-in-windows-10/)
+ [MacOS](https://superuser.com/questions/498943/directory-of-url-schemes-for-mac-apps)
+ [Linux](https://wiki.archlinux.org/title/XDG_MIME_Applications#Shared_MIME_database)




### `system` Entry

A system entry allows the user to run an arbitrary program using their system's shell. 

For example:

```toml
# open system calculator (windows specific)
['Open Calculator']
system = 'calc.exe'
```

In this entry, the calculator program will be opened. (This example
works for Windows: for other operating systems you will need to
replace the executable with OS's specific calculator program).

### <a name="keyword"></a> `keyword` Entry


`keyword` entries are a little bit different than the other type of
Jolly entries. Instead of pointing to a specific destination, a
keyword entry is allowed to have a parameter which is included in the target. 

This is similar to [keyword
bookmarks](https://www-archive.mozilla.org/docs/end-user/keywords.html)
or [custom search
engines](https://support.google.com/chrome/answer/95426), in a web
browser.

To specify that a Jolly entry is a `keyword` entry, just include an
extra key in the entry table that describes what shortcut to
use. Then, in the *entry name* or *entry target*, you can use the
string `%s` to indicate where the parameter for the keyword should be
inserted:

```toml
['Search DuckDuckGo: %s']
location = 'https://duckduckgo.com/?q=%s'
keyword = 'ddg'
escape = true
```

In this example, we can type the text `ddg` into the Jolly search
window, followed by a space, and then whatever is typed afterwards
will be used to search the web using DuckDuckGo. 

You will notice that this entry has another key in it that we haven't
talked about yet: `escape`. By default, Jolly will put whatever text
you type in the window into the `%s` parameter location in the
target. This works okay for some entries, but web urls typically need
to be [percent
encoded](https://en.wikipedia.org/wiki/Percent-encoding). You can have
Jolly percent-encode the keyword parameter by including the key entry `escape`.

*Search Order* If the user types a string of text that matches the
shortcut for a `keyword entry`, Jolly will rank this as the most
relevant search result. This will bypass any other entries even if
they have a better score based on tags. For more details, see the
search algorithm documentation. 

### `url` Entry


Syntatic sugar for specifying an `escape = true` `location` entry. 


A `url` entry is almost exactly like a location entry, except that if
the url is used as a keyword entry, it defaults to having the keyword
parameter be percent encoded. If you want this entry to be a keyword
entry, and it is pointing at a website, you will generally want to use
a url entry instead of a location entry.

The previous example of a keyword entry could therefore be written
more compactly like this:

```toml
['Search DuckDuckGo: %s']
url = 'https://duckduckgo.com/?q=%s'
keyword = 'ddg'
```

# <a name="locations"></a> Jolly Database Search Locations


Jolly searches for a database file (always named `jolly.toml` in the
following locations, in descending order of priority:

1. The current working directory
2. In the User's [configuration directory](https://docs.rs/dirs/latest/dirs/fn.config_dir.html)

*Note: The user's configuration directory is platform dependent, the
value for various platforms can be found in the above link.*

# <a name="errors"></a> Errors
Sometimes Jolly will encounter an error can cannot proceed. Usually,
in this situation, the normal Jolly window will still show, but the
search box will be read only, and the text of the error will be shown
in the box. Below are some descriptions of some errors that you might
see.

## <a name="error-toml"></a> TOML Error
If the `jolly.toml` file contains a syntax error, and is not a valid
TOML file, then jolly will show an error window instead of the normal startup screen. 

![toml-error](static/toml-error.png)

Fix the syntax error in order to proceed.
