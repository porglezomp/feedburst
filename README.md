# Feedburst!

[![Build status](https://ci.appveyor.com/api/projects/status/wsg83k3i456yi32s?svg=true)](https://ci.appveyor.com/project/porglezomp/feedburst)
[![Build Status](https://travis-ci.org/porglezomp/feedburst.svg)](https://travis-ci.org/porglezomp/feedburst)
[![Coverage Status](https://coveralls.io/repos/github/porglezomp/feedburst/badge.svg?branch=develop)](https://coveralls.io/github/porglezomp/feedburst?branch=develop)
[![Release](https://img.shields.io/github/release/porglezomp/feedburst.svg)](https://github.com/porglezomp/feedburst/releases/latest)
[![Crates.io](https://img.shields.io/crates/v/feedburst.svg)](https://crates.io/crates/feedburst)

Feedburst is a tool that presents you your RSS feeds in chunks, according to a policy that you set.

## Installing

You can install Feedburst by going to the releases page and downloading the latest release for your platform.
If you have `cargo` already installed, you can also get it by running:

```
cargo install feedburst
```

## Configuring

Feedburst is configured with a config file containing all the comics you'd like to read, and policy about when and how you'd like to read them.
Any line that begins with with a `#` will be treated as a comment and ignored.
Entries in that config file look like the following:

```
# A nice friendly comic
"Goodbye to Halos" <http://goodbyetohalos.com/feed/> @ 2 new comics @ overlap 1 comic @ on monday
```

The `"Title"` is whatever title you’d like to display the comic as.
The `<link>` is a link to the RSS feed to pull the comics from.
The `@policy` are rules for when and how you’d like that comic feed to be presented to you.

- `@ # new comic(s)`: Wait for there to be at least # new comics before you see them.
- `@ overlap # comic(s)`: Show the last # comics that you read.
- `@ on monday/tuesday/etc…`: Show the comics once the corresponding day has passed.
- `@ every # day(s)`: Wait at least # days since you last read the comic.
- `@ ignore url /pattern/`: Don't include comics that have `pattern` in the URL (also `ignore title`).
- `@ keep title /pattern/`: Only include comics that have `pattern` in the title (also `keep url`).
- `@ open all`: Open every new comic, not just the earliest. This is useful for some tumblr comics that don't have forward/backward buttons on individual pages.

For more features, [see the advanced config section](#advanced-config).

## Config Location

By default, on macOS and Linux, the config file is stored at:

```
~/.config/feedburst/config.feeds
```

and on Windows, it's stored at

```
%AppData%\Feedburst\config.feeds
```

If you want to set a different default location for your config file, you can set the `$FEEDBURST_CONFIG_FILE` environment variable.
If you want to use a different config for a single run, then use `--config FILE` on the command line.

## Advanced Config

### Feed Data Location

By default, all of your feeds are stored together.
On macOS and Linux, they're stored at:

```
~/.local/share/feedburst/feeds/
```

On Windows you can find your feeds at:

```
%AppData%\Feedburst\feeds\
```

If you want to store your feeds in a location different from the default, then you have two options.
First, you can override the base path for all of your comics on the command line with `--feeds PATH`.
If you'd like to permanently change the base path, then add a line to your config file

```
root PATH
```

This will store all feeds that come after that line at `PATH`.
You can use as many `root` directives as you want to, and each feed will use whichever was specified most recently.
If you'd like to reset later feeds to be stored at the default location, then just put `feed` on its own on the line.

### Customizing the Browser

By default feedburst will try to open comics in your default browser.
If that doesn't work, or if you want to open your comics in another browser, you can customize the command it uses to open it using `command` in your config file.
Any comics that come after that line will be opened using that command.

For example,
```
command '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' --incognito
```
on macOS will use Chrome to open the comic in Incognito mode.

If you'd like to reset later feeds to be opened with the default command, just put `command` on its own line.
