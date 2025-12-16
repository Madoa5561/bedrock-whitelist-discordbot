use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateActionRow, CreateInputText, InputTextStyle,
    CreateModal, ModalInteraction,
};
use crate::server_controller::ServerController;
use std::sync::Arc;
use std::env;

pub fn register(name: &str, description: &str) -> CreateCommand {
    CreateCommand::new(name).description(description)
}

pub async fn handle_command(
    ctx: &Context, 
    interaction: &CommandInteraction, 
    server_controller: Arc<ServerController>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let lang = env::var("LANGUAGE").unwrap_or_else(|_| "JP".to_string());
    let is_en = lang.to_uppercase() == "EN";
    match interaction.data.name.as_str() {
        "server" => {
            let (title, label, placeholder) = if is_en {
                ("Server Registration", "Game ID", "Enter your Game ID")
            } else {
                ("サーバー登録", "ゲームID", "ゲームIDを入力してください")
            };
            let modal = CreateModal::new("server_modal", title)
                .components(vec![
                    CreateActionRow::InputText(
                        CreateInputText::new(InputTextStyle::Short, label, "game_id")
                            .placeholder(placeholder)
                            .required(true)
                    )
                ]);

            interaction
                .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
                .await?;
        }
        "restart" => {
            // -------------------------
            // 現在restartコマンドの実行権限は限定されていません、everyoneに実行できるようになっています
            // あなたがもしこのコードをそのまま使用する場合は **絶対に** restartを削除するか権限を限定するようにコードを編集してください
            // -------------------------
            let msg = if is_en { "🔄 Restarting server..." } else { "🔄 サーバーを再起動しています..." };
            interaction
                .create_response(
                    &ctx.http, 
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(msg).ephemeral(false)
                    )
                )
                .await?;
            let controller = Arc::clone(&server_controller);
            tokio::task::spawn_blocking(move || {
                if let Err(e) = controller.restart() {
                    eprintln!("Failed to restart server: {}", e);
                }
            });
        }
        _ => {}
    }
    Ok(())
}

pub async fn handle_modal(
    ctx: &Context, 
    interaction: &ModalInteraction, 
    server_controller: Arc<ServerController>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let lang = env::var("LANGUAGE").unwrap_or_else(|_| "JP".to_string());
    let is_en = lang.to_uppercase() == "EN";
    let game_id = interaction
        .data
        .components
        .first()
        .and_then(|row| row.components.first())
        .and_then(|component| {
            if let serenity::all::ActionRowComponent::InputText(input) = component {
                input.value.clone()
            } else {
                None
            }
        })
        .unwrap_or_default();
    if game_id.is_empty() {
        let msg = if is_en { "❌ Please enter a Game ID." } else { "❌ ゲームIDを入力してください。" };
        let response = CreateInteractionResponseMessage::new()
            .content(msg)
            .ephemeral(true);
        
        interaction
            .create_response(&ctx.http, CreateInteractionResponse::Message(response))
            .await?;
        return Ok(());
    }
    match server_controller.send_command(&format!("allowlist add \"{}\"", game_id)) {
        Ok(_) => {
             let msg = if is_en {
                format!("✅ `{}` has been added to the whitelist!", game_id)
            } else {
                format!("✅ `{}` をホワイトリストに追加しました！", game_id)
            };
            let response = CreateInteractionResponseMessage::new()
                .content(msg)
                .ephemeral(true);
            
            interaction
                .create_response(&ctx.http, CreateInteractionResponse::Message(response))
                .await?;
        }
        Err(e) => {
            eprintln!("Error sending command: {}", e);
             let msg = if is_en {
                "❌ Failed to send command to server."
            } else {
                "❌ サーバーへのコマンド送信に失敗しました。"
            };
            let response = CreateInteractionResponseMessage::new()
                .content(msg)
                .ephemeral(true);
            
            interaction
                .create_response(&ctx.http, CreateInteractionResponse::Message(response))
                .await?;
        }
    }
    Ok(())
}

