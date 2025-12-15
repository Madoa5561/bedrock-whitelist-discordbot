use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateActionRow, CreateInputText, InputTextStyle,
    CreateModal, ModalInteraction,
};
use crate::allowlist::AllowlistManager;
use std::sync::Arc;
use std::env;

pub fn register() -> CreateCommand {
    let lang = env::var("LANGUAGE").unwrap_or_else(|_| "JP".to_string());
    let description = if lang.to_uppercase() == "EN" {
        "Register to the Minecraft server allowlist"
    } else {
        "Minecraftサーバーのallowlistに登録する"
    };

    CreateCommand::new("server")
        .description(description)
}

pub async fn handle_command(ctx: &Context, interaction: &CommandInteraction, _allowlist: Arc<AllowlistManager>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let lang = env::var("LANGUAGE").unwrap_or_else(|_| "JP".to_string());
    let is_en = lang.to_uppercase() == "EN";

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

    Ok(())
}

pub async fn handle_modal(ctx: &Context, interaction: &ModalInteraction, allowlist: Arc<AllowlistManager>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    match allowlist.add_entry(game_id.clone()).await {
        Ok(true) => {
            let msg = if is_en {
                format!("✅ `{}` has been added to the allowlist!", game_id)
            } else {
                format!("✅ `{}` をallowlistに追加しました!", game_id)
            };
            let response = CreateInteractionResponseMessage::new()
                .content(msg)
                .ephemeral(true);
            
            interaction
                .create_response(&ctx.http, CreateInteractionResponse::Message(response))
                .await?;
        }
        Ok(false) => {
            let msg = if is_en {
                format!("⚠️ `{}` is already in the allowlist.", game_id)
            } else {
                format!("⚠️ `{}` は既にallowlistに登録されています。", game_id)
            };
            let response = CreateInteractionResponseMessage::new()
                .content(msg)
                .ephemeral(true);
            
            interaction
                .create_response(&ctx.http, CreateInteractionResponse::Message(response))
                .await?;
        }
        Err(e) => {
            eprintln!("Error adding to allowlist: {}", e);
            let msg = if is_en {
                "❌ An error occurred while adding to the allowlist."
            } else {
                "❌ allowlistへの追加中にエラーが発生しました。"
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
