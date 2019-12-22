use crate::{
    log_error,
    opt::{Command, FeedType, Opt},
    stream::{Game, LazyStream, Stream},
    BANNER,
};
use async_std::{process, task};
use chrono::Local;
use failure::Error;
use read_input::prelude::*;

pub fn run(opts: Opt) {
    task::block_on(async {
        if let Err(e) = process(&opts, false).await {
            log_error(&e);
            process::exit(1);
        };
    });

    if cfg!(target_os = "windows") {
        pause();
    }
}

pub async fn process(opts: &Opt, need_return: bool) -> Result<(Game, Stream), Error> {
    println!("{}", BANNER);

    let resolve = if let Command::Select { resolve } = opts.command {
        resolve
    } else {
        false
    };

    let lazy_stream = LazyStream::new(opts).await?;
    let mut games = lazy_stream.games();

    println!(
        "\nPick a game for {}...\n",
        lazy_stream.date().format("%Y-%m-%d")
    );
    for (idx, game) in games.iter().enumerate() {
        println!(
            "{}) {} - {} @ {}",
            idx + 1,
            game.game_date
                .with_timezone(&Local)
                .time()
                .format("%-I:%M %p")
                .to_string(),
            game.away_team.name,
            game.home_team.name
        );
    }

    let game_count = games.len();
    let game_choice = input::<usize>()
        .msg("\n>>> ")
        .add_test(move |input| *input > 0 && *input <= game_count)
        .get();
    let mut game = games.remove(game_choice - 1);

    let mut streams = game.streams().await?;

    println!("\nPick a stream...\n");

    let mut feeds: Vec<FeedType> = streams.clone().into_iter().map(|(k, _)| k).collect();
    feeds.sort();
    for (idx, feed_type) in feeds.iter().enumerate() {
        println!("{}) {}", idx + 1, feed_type);
    }

    let feed_count = feeds.len();
    let feed_choice = input::<usize>()
        .msg("\n>>> ")
        .add_test(move |input| *input > 0 && *input <= feed_count)
        .get();
    let feed_choice = &feeds[(feed_choice - 1)];
    let mut stream = streams.remove(feed_choice).unwrap();

    let host_link = stream.host_link(&lazy_stream.opts.cdn);

    let cdn = &lazy_stream.opts.cdn;
    if !need_return {
        println!();
        if let Some(ref quality) = lazy_stream.opts.quality {
            let quality_link = stream.quality_link(cdn, quality).await?;
            println!("{}", quality_link);
        } else if resolve {
            let master_link = stream.master_link(cdn).await?;
            println!("{}", master_link);
        } else {
            println!("{}", host_link);
        }
    }

    Ok((game, stream))
}

// Keep console window open until button press
fn pause() {
    use std::io::{self, prelude::*};

    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    write!(stdout, "\nPress enter or close window to exit...").unwrap();
    stdout.flush().unwrap();

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
}
