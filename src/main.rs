mod allowlist;
mod commands;
mod status;

use allowlist::AllowlistManager;
use serenity::all::{
    Command, Context, EventHandler, GatewayIntents, Interaction, Ready,
};
use serenity::Client;
use std::env;
use std::sync::Arc;
use status::StatusMonitor;

struct Handler {
    allowlist: Arc<AllowlistManager>,
    status_monitor: Arc<StatusMonitor>,
}

impl Handler {
    fn new(allowlist: Arc<AllowlistManager>, status_monitor: Arc<StatusMonitor>) -> Self {
        Self {
            allowlist,
            status_monitor,
        }
    }
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        
        let commands = vec![commands::register()];
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
                if command.data.name == "server" {
                    if let Err(e) = commands::handle_command(&ctx, &command, Arc::clone(&self.allowlist)).await {
                        eprintln!("Error handling command: {}", e);
                    }
                }
            }
            Interaction::Modal(modal) => {
                if modal.data.custom_id == "server_modal" {
                    if let Err(e) = commands::handle_modal(&ctx, &modal, Arc::clone(&self.allowlist)).await {
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
    let allowlist_path = env::var("ALLOWLIST_PATH")
        .unwrap_or_else(|_| "../allowlist.json".to_string());

    let allowlist = Arc::new(AllowlistManager::new(allowlist_path));
    
    let server_ip = env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "19132".to_string())
        .parse::<u16>()
        .expect("SERVER_PORT must be a valid u16");

    let status_monitor = Arc::new(StatusMonitor::new(
        channel_id,
        server_ip,
        server_port,
    ));

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler::new(allowlist, status_monitor))
        .await
        .expect("Error creating client");

    println!("Starting bot...");
    if let Err(why) = client.start().await {
        eprintln!("Client error: {:?}", why);
    }
}
