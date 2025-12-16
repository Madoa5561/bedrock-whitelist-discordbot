mod commands;
mod status;
mod server_controller;

use serenity::all::{
    Command, Context, EventHandler, GatewayIntents, Interaction, Ready,
};
use serenity::Client;
use std::env;
use std::sync::Arc;
use status::StatusMonitor;
use server_controller::ServerController;

struct Handler {
    server_controller: Arc<ServerController>,
    status_monitor: Arc<StatusMonitor>,
}

impl Handler {
    fn new(server_controller: Arc<ServerController>, status_monitor: Arc<StatusMonitor>) -> Self {
        Self {
            server_controller,
            status_monitor,
        }
    }
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let lang = env::var("LANGUAGE").unwrap_or_else(|_| "JP".to_string());
        let is_en = lang.to_uppercase() == "EN";
        let server_desc = if is_en { "Register to the Minecraft server allowlist" } else { "Minecraftサーバーのallowlistに登録する" };
        let restart_desc = if is_en { "Restart the Minecraft server" } else { "Minecraftサーバーを再起動する" };

        // -------------------------
        // 現在restartコマンドの実行権限は限定されていません、everyoneに実行できるようになっています
        // あなたがもしこのコードをそのまま使用する場合は **絶対に** restartを削除するか権限を限定するようにコードを編集してください
        let commands = vec![
            commands::register("server", server_desc),
            commands::register("restart", restart_desc),
        ];
        // -------------------------

        if let Err(e) = Command::set_global_commands(&ctx.http, commands).await {
            eprintln!("Error registering commands: {}", e);
        } else {
            println!("Slash commands registered successfully!");
        }

        Arc::clone(&self.status_monitor).start(ctx).await;
        println!("Status monitoring started!");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                if let Err(e) = commands::handle_command(&ctx, &command, Arc::clone(&self.server_controller)).await {
                    eprintln!("Error handling command: {}", e);
                }
            }
            Interaction::Modal(modal) => {
                if modal.data.custom_id == "server_modal" {
                    if let Err(e) = commands::handle_modal(&ctx, &modal, Arc::clone(&self.server_controller)).await {
                        eprintln!("Error handling modal: {}", e);
                    }
                }
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");
    let channel_id = env::var("STATUS_CHANNEL_ID")
        .expect("Expected STATUS_CHANNEL_ID in environment")
        .parse::<u64>()
        .expect("STATUS_CHANNEL_ID must be a valid u64");
    let server_path = env::var("SERVER_PATH").unwrap_or_else(|_| "../".to_string());
    let server_controller = Arc::new(ServerController::new(server_path));
    if let Err(e) = server_controller.start() {
        eprintln!("Failed to start bedrock_server: {}", e);
        return;
    }

    let display_ip = env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let connect_ip = env::var("INTERNAL_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "19132".to_string())
        .parse::<u16>()
        .expect("SERVER_PORT must be a valid u16");

    let status_monitor = Arc::new(StatusMonitor::new(
        channel_id,
        display_ip,
        connect_ip,
        server_port,
    ));

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler::new(Arc::clone(&server_controller), status_monitor))
        .await
        .expect("Error creating client");
    {
        let console_controller = Arc::clone(&server_controller);
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            let mut line = String::new();
            loop {
                if stdin.read_line(&mut line).is_err() {
                    break;
                }
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    if let Err(e) = console_controller.send_command(trimmed) {
                        eprintln!("Failed to send command: {}", e);
                    }
                }
                line.clear();
            }
        });
    }
    let shutdown_signal = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            eprintln!("Failed to listen for Ctrl+C: {}", e);
        }
        println!("\nShutdown signal received. Stopping server...");
        server_controller.stop();
        println!("Cleanup complete. Exiting.");
        std::process::exit(0);
    };

    println!("Starting bot and server monitor...");
    tokio::select! {
        result = client.start() => {
            if let Err(why) = result {
                eprintln!("Client error: {:?}", why);
            }
        },
        _ = shutdown_signal => {},
    }
}
