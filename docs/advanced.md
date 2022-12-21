Below are a couple of advanced tips and tricks for using Jolly:

# Copying Links

Sometimes you don't need to open a Jolly entry, just determine the location
that that entry points to. 

You can have Jolly copy the entry target to your system clipboard by
holding down the Control key (Command on MacOS) when selecting the
entry:

![copying](static/clipboard.png)

# Entry Ranking Algorithm

Below are some details about how the Jolly chooses to rank and display entries. 

Searches are presumed to be case insensitive, unless the search query
has an uppercase letter in it, in which case the ranking is done in a
case sensitive manner.

Each entry in the configuration file is assigned a score based on how
well its title and tags match the search query.

Ties in score are broken by reverse entry order in the
[jolly.toml](file-format.md) file. That is, the later an entry appears
in the `jolly.toml` file, the higher its appearance in the results
list. This is because we assume that users will add newer entries to
the bottom of the configuration file, and new entries should be ranked
higher than older ones.

## Score Calculation

First, the search query is split into tokens based on [whitespace
boundaries](https://doc.rust-lang.org/std/primitive.str.html#method.split_whitespace).

Each token of the search query is considered to be an additional
filter on the search results: That is, each token is ANDed together to
only show results that best match all of the search tokens. 

The score for each token is calculated by seeing how well it matches each of the following heuristics:

| Heuristic Name   | Current Weight | Description of Heuristic                                     |
|------------------|----------------|--------------------------------------------------------------|
| FULL_KEYWORD_W   | 100            | Does the first token exactly match this entry's keyword tag? |
| PARTIAL_NAME_W   | 3              | Does the entry name contain this token?                      |
| FULL_NAME_W      | 10             | Does the entry name match this token?                        |
| PARTIAL_TAG_W    | 2              | Do any of the entry's tags contain this token?               |
| STARTSWITH_TAG_W | 4              | Do any of the entry's tags start with this token?            |
| FULL_TAG_W       | 6              | Do any of the entry's tags match this token?                 |

The best score from each of these heuristics is chosen for each token,
and then the minimum score from each token is taken as the overall
score for the entry.


Note: the keyword entry heuristic is a special case, since it is only
calculated for the first token. If the entry is a [keyword
entry](file-format.md#keyword), and the first token in the search
query matches the keyword key, the results is assigned a fixed score
`FULL_KEYWORD_W`.
