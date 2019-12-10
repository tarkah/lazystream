# LazyStream
[![Build Status](https://dev.azure.com/tarkah/lazystream/_apis/build/status/tarkah.lazystream?branchName=master)](https://dev.azure.com/tarkah/lazystream/_build/latest?definitionId=11&branchName=master)

Easily get LazyMan stream links, output directly or to m3u / xmltv formats.

- Defaults to grabbing the current days games. `--date YYYYMMDD` can be specified for a certain day. 
- An m3u playlist can be generated for all games with the `--playlist-output` option
- An xmltv file with corresponding m3u playlist can be generated with the `--xmltv-output` option

```
❯ lazystream --help

lazystream 1.4.0
tarkah <admin@tarkah.dev>
Easily get LazyMan stream links, output directly or to m3u / xmltv formats.

USAGE:
    lazystream [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --date <YYYYMMDD>                              Specify what date to generate stream links for, defaults to today
        --playlist-output <playlist-output>            Generate a .m3u playlist file for all games
        --xmltv-output <xmltv-output>
            Generate a .xml XMLTV file for all games with corresponding .m3u playlist file

        --xmltv-start-channel <xmltv-start-channel>
            Specify the starting channel number for the XMLVTV output [default: 1000]

❯ lazystream

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