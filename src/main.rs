mod board_games;

use std::{error::Error, sync::Arc};
use board_games::BoardGame;

use serde_json::json;
use teloxide::{prelude::*, types::Me, utils::command::BotCommands};
use tokio::sync::oneshot;
use warp::Filter;
use std::env;

use libsql::{Builder, Database};

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
    let shutdown_server_tx = setup_healthcheck();

    let (board_games, db) = fetch_board_games().await.expect("Failed to fetch board games");
    println!("Loaded {} board games", board_games.len());

    let dba: Arc<Database> = Arc::new(db);

    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(handler_with_metrics));
        // .branch(Update::filter_callback_query().endpoint(callback_handler))
        // .branch(Update::filter_inline_query().endpoint(inline_query_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![board_games, dba])
        .enable_ctrlc_handler().build().dispatch().await;

    // start the shutdown...
    let _ = shutdown_server_tx.send(());
    println!("Shutting down...");

    Ok(())
}

/// Parse the text wrote on Telegram and check if that text is a valid command
async fn handler_with_metrics(
    bot: Bot,
    msg: Message,
    me: Me,
    board_games: Vec<BoardGame>,
    dba: Arc<Database>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let start = std::time::Instant::now();
    let text = msg.text().unwrap_or("");
    let chat_id = msg.chat.id.0;
    let result = message_handler(bot, msg.clone(), me, board_games).await; // Pass the cloned message
    let response_time = start.elapsed().as_millis().try_into().unwrap_or(-1);
    let bot_id = "bastard_cafe_bot";
    let ok = result.is_ok();
    let user_hash = msg.from()
        .map(|user| format!("{}:{}", user.id.0, user.username.clone().unwrap_or("unknown".to_string())))
        .unwrap_or("0:unknown".to_string());

    let details = match result {
        Ok(len_or_code) => match len_or_code {
            -1 => json!({"code": -1, "command": "Help", "response_type": "Help command", "user": user_hash}),
            -2 => json!({"code": -2, "command": "search", "response_type": "Search term too short", "user": user_hash}),
            -666 => json!({"code": -666, "command": "unknown", "response_type": "should not happen", "user": user_hash}),
            _ => json!({"code": 0, "command": "search", "response_type": "Search results", "total": len_or_code, "user": user_hash}),
        },
        Err(e) => json!({"code": -1000, "command": "unknown", "response_type": "Error", "error": e.to_string()})
    };
    let details_txt = serde_json::to_string(&details).unwrap();

    let conn = dba.connect();
    match conn {
        Err(e) => {
            eprintln!("Failed to connect to database: {}", e);
            return Ok(());
        },
        Ok(conn) => {
            let query = "INSERT INTO bot_metrics (bot_id, chat_id, text, ok, details, response_time_ms, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))";
            let db_result = conn.execute(query, libsql::params![bot_id, chat_id, text, ok, details_txt.clone(), response_time]).await;
            let ok_txt = if ok { "ok" } else { "error" };
            println!("[{}] [{}] [{}ms] [{}]: {} {} {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), bot_id, response_time, ok_txt, chat_id, text, details_txt);
            if let Err(e) = db_result {
                eprintln!("Failed to insert metrics: {}", e);
            }
        }
    }

    

    return Ok(());
}


/// Parse the text wrote on Telegram and check if that text is a valid command
async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
    board_games: Vec<BoardGame>
) -> Result<i32, Box<dyn Error + Send + Sync>> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Help) => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
                return Ok(-1);
            },
            Ok(Command::Search(term)) | Ok(Command::S(term)) => {
                return search_handler(bot, term, msg.chat.id, board_games).await;
            },
            Err(_) => {
                return search_handler(bot, text.to_string(), msg.chat.id, board_games).await;
            },
        };
    }
    Ok(-666)
}


async fn search_handler(bot: Bot, term: String, chat_id: ChatId, board_games: Vec<BoardGame>) -> Result<i32, Box<dyn Error + Send + Sync>> {
    let term = term.trim();
    if term.len() < 3 {
        bot.send_message(chat_id, "Search term must be at least 3 characters long").await?;
        return Ok(-2);
    }

    let games = board_games.iter().filter(|game| game.name.to_lowercase().contains(&term.to_lowercase())).collect::<Vec<&BoardGame>>();
    // let games: Vec<BoardGame> = vec![];
    if games.is_empty() {
        bot.send_message(chat_id, "No games found").await?;
        return Ok(0);
    }
    let games_str = games.iter().map(|game| game.human_friendly()).collect::<Vec<String>>().join("\n\n");
    bot.send_message(chat_id, games_str).await?;
    return Ok(games.len() as i32);
}



// DATABASE FETCHING
async fn fetch_board_games() -> Result<(Vec<BoardGame>, Database), Box<dyn std::error::Error>> {
    let url = env::var("LIBSQL_DATABASE_URL").expect("LIBSQL_DATABASE_URL must be set");
    let token = env::var("LIBSQL_AUTH_TOKEN").expect("LIBSQL_AUTH_TOKEN must be set");

    let db = Builder::new_remote(url, token).build().await?;
    let conn = db.connect().unwrap();
    let mut result = conn.query("SELECT id, created_at FROM batch WHERE status = 'DONE' AND batch_type = 'BOARD_GAME_LIST' ORDER BY id DESC LIMIT 1", libsql::params![]).await?;
    let row = result.next().await.unwrap().unwrap();

    let value = row.get_value(0).unwrap();
    let most_recent_batch_id = value.as_integer().unwrap();
    let created_at_value = row.get_value(1).unwrap();
    let created_at = created_at_value.as_text().unwrap();

    println!("Downloading most recent batch: {} - {}", most_recent_batch_id, created_at);

    let mut board_game_rows = conn.query("SELECT value FROM key_value WHERE batch_id = ?1", libsql::params![most_recent_batch_id]).await?;

    let mut board_games = Vec::new();
    
    while let Some(row) = board_game_rows.next().await? {
        let value = row.get_value(0).unwrap();
        let bg_text = value.as_text().unwrap();
        let board_game: BoardGame = serde_json::from_str(&bg_text)?;
        board_games.push(board_game);
    }

    println!("Downloaded {} board games", board_games.len());
    println!("first game: {:?}", board_games.first().unwrap());

    Ok((board_games, db))
}



// HEALTH CHECK ENDPOINT
fn setup_healthcheck() -> oneshot::Sender<()> {
    let health_check = warp::path!("health")
    .map(|| warp::reply::json(&"ok"));

    let (tx, rx) = oneshot::channel();

    let (addr, server) = warp::serve(health_check)
        .bind_with_graceful_shutdown(([0,0,0,0], 8080), async {
            rx.await.ok();
        });


    // Spawn the server into a runtime
    tokio::task::spawn(server);
    println!("Listening on http://{}/health", addr);

    return tx;
}
