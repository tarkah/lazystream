# LazyStream
[![Build Status](https://dev.azure.com/tarkah/lazystream/_apis/build/status/tarkah.lazystream?branchName=master)](https://dev.azure.com/tarkah/lazystream/_build/latest?definitionId=11&branchName=master)

Easily get LazyMan stream links, output directly or to m3u / xmltv formats.

- Defaults to grabbing the current days games. `--date YYYYMMDD` can be specified for a certain day. 
- xmltv and m3u playlist formats can be generated for all games using the `generate` subcommand
- Games can be recorded using the `record` subcommand. This requires StreamLink is installed and in your path. If a game is live, you can use the `--restart` flag to start recording from the beginning of the stream. Currently `best` quality is used with StreamLink, the `--quality` option doesn't affect this, for now.
- Games can be casted to a chromecast using the `cast` subcommand. In addition to Streamlink, VLC is required to cast the stream.
- Play games directly to VLC with the `play` subcommand. Requires both Streamlink and VLC.

```
❯ lazystream --help

lazystream 1.8.1
tarkah <admin@tarkah.dev>
Easily get LazyMan stream links, output directly or to m3u / xmltv formats. Streams can also be recorded or casted.

USAGE:
    lazystream [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --date <YYYYMMDD>      Specify what date to use for games, defaults to today
        --cdn <cdn>            Specify which CDN to use: 'akc' or 'l3c' [default: akc]
        --quality <quality>    Specify a quality to use, otherwise stream will be adaptive. Doesn't currently affect
                               recording

SUBCOMMANDS:
    select      Select stream link via command line
    generate    Generate an xmltv and/or playlist formatted output for all games
    play        Play a game with VLC, requires StreamLink and VLC
    record      Record a game, requires StreamLink
    cast        Cast a game, requires StreamLink and VLC
    help        Prints this message or the help of the given subcommand(s)

❯ lazystream select

 |        \   __  /\ \   / ___|__ __|  _ \  ____|    \     \  | 
 |       _ \     /  \   /\___ \   |   |   | __|     _ \   |\/ | 
 |      ___ \   /      |       |  |   __ <  |      ___ \  |   | 
_____|_/    _\____|   _| _____/  _|  _| \_\_____|_/    _\_|  _| 


Pick a game for 2019-12-09...

1) 4:00 PM - Chicago Blackhawks @ Boston Bruins
2) 4:00 PM - Colorado Avalanche @ MontrÃ©al Canadiens
3) 4:00 PM - Minnesota Wild @ Tampa Bay Lightning
4) 4:00 PM - Vegas Golden Knights @ New York Islanders
5) 4:00 PM - Arizona Coyotes @ Philadelphia Flyers
6) 4:00 PM - San Jose Sharks @ Carolina Hurricanes
7) 4:00 PM - New York Rangers @ Columbus Blue Jackets
8) 5:30 PM - Winnipeg Jets @ Dallas Stars
9) 6:00 PM - Buffalo Sabres @ Calgary Flames

>>> 4

Pick a stream...

1) HOME
2) AWAY
3) COMPOSITE

>>> 2

http://nhl.freegamez.ga/getM3U8.php?league=nhl&date=2019-12-05&id=70395003&cdn=akc
```