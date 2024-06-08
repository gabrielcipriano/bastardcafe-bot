mod board_games;

use std::fs;

use board_games::BoardGame;
// use std::collections::HashMap;

// use tokio::sync::Mutex;
// use std::sync::Arc;

use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "Searches for a board game in the bastard's shelfs. An alias for /search")]
    Search(String),
    S(String),
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load board games from json
    let content: String = fs::read_to_string("./data/board_games_list.json")?;
    let board_games: Vec<BoardGame> = serde_json::from_str(&content)?;

    println!("Loaded {} board games", board_games.len());

    let bot = Bot::from_env();

    let cmd_handler = move |bot: Bot, msg: Message, cmd: Command| {
        let board_games = board_games.clone();
        async move {
            match cmd {
                Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
                Command::Search(term) => {
                    let term = term.trim();
                    if term.len() < 3 {
                        bot.send_message(msg.chat.id, "Search term must be at least 3 characters long").await?;
                        return Ok(());
                    }

                    let games = board_games.iter().filter(|game| game.name.to_lowercase().contains(&term.to_lowercase())).collect::<Vec<&BoardGame>>();
                    if games.is_empty() {
                        bot.send_message(msg.chat.id, "No games found").await?;
                        return Ok(());
                    }
                    let games_str = games.iter().map(|game| game.human_friendly()).collect::<Vec<String>>().join("\n\n");
                    bot.send_message(msg.chat.id, games_str).await?;
                    return Ok(());
                }
                Command::S(term) => {
                    let term = term.trim();
                    if term.len() < 3 {
                        bot.send_message(msg.chat.id, "Search term must be at least 3 characters long").await?;
                        return Ok(());
                    }

                    let games = board_games.iter().filter(|game| game.name.to_lowercase().contains(&term.to_lowercase())).collect::<Vec<&BoardGame>>();
                    if games.is_empty() {
                        bot.send_message(msg.chat.id, "No games found").await?;
                        return Ok(());
                    }
                    let games_str = games.iter().map(|game| game.human_friendly()).collect::<Vec<String>>().join("\n\n");
                    bot.send_message(msg.chat.id, games_str).await?;
                    return Ok(());
                }
            };
            Ok(())
        }
    };

    Command::repl(bot, cmd_handler).await;

    // let bot = Bot::from_env();

    // teloxide::repl(bot, |bot: Bot, msg: Message| async move {
    //     bot.send_dice(msg.chat.id).await?;
    //     Ok(())
    // })
    // .await;

    Ok(())
}