use byteorder::{BigEndian, WriteBytesExt};
use chrono::Local;
use serenity::all::{ChannelId, Context, CreateMessage, EditMessage, MessageFlags, MessageId};
use std::io::Cursor;
use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, timeout};
use std::env;

pub struct StatusMonitor {
    channel_id: ChannelId,
    server_ip: String,
    server_port: u16,
    last_message_id: Arc<RwLock<Option<MessageId>>>,
}

impl StatusMonitor {
    pub fn new(channel_id: u64, server_ip: String, server_port: u16) -> Self {
        Self {
            channel_id: ChannelId::new(channel_id),
            server_ip,
            server_port,
            last_message_id: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start(self: Arc<Self>, ctx: Context) {
        let ctx = Arc::new(ctx);
        
        if let Err(e) = self.cleanup_old_messages(&ctx).await {
            eprintln!("Error cleaning up old messages: {}", e);
        }
        
        let status_ctx = Arc::clone(&ctx);
        let status_self = Arc::clone(&self);
        tokio::spawn(async move {
            status_self.update_loop(status_ctx).await;
        });

        let message_ctx = Arc::clone(&ctx);
        let message_self = Arc::clone(&self);
        tokio::spawn(async move {
            message_self.message_monitor(message_ctx).await;
        });
    }

    async fn cleanup_old_messages(&self, ctx: &Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bot_id = ctx.cache.current_user().id;
        
        if let Ok(messages) = self.channel_id.messages(&ctx.http, Default::default()).await {
            for message in messages {
                if message.author.id == bot_id && (message.content.contains("Minecraft Bedrock サーバー状態") || message.content.contains("Minecraft Bedrock Server Status")) {
                    if let Err(e) = self.channel_id.delete_message(&ctx.http, message.id).await {
                        eprintln!("Failed to delete old message {}: {}", message.id, e);
                    } else {
                        println!("Deleted old status message: {}", message.id);
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn update_loop(self: Arc<Self>, ctx: Arc<Context>) {
        loop {
            if let Err(e) = self.update_status(&ctx).await {
                eprintln!("Error updating status: {}", e);
            }
            sleep(Duration::from_secs(30)).await;
        }
    }

    async fn message_monitor(self: Arc<Self>, ctx: Arc<Context>) {
        let mut last_check_message_id = {
            let messages = self.channel_id.messages(&ctx.http, Default::default()).await.ok();
            messages.and_then(|msgs| msgs.first().map(|m| m.id))
        };

        loop {
            sleep(Duration::from_secs(5)).await;

            if let Ok(messages) = self.channel_id.messages(&ctx.http, Default::default()).await {
                if let Some(latest) = messages.first() {
                    let our_message_id = self.last_message_id.read().await;
                    
                    if Some(latest.id) != *our_message_id && Some(latest.id) != last_check_message_id {
                        drop(our_message_id);
                        if let Err(e) = self.repost_status(&ctx).await {
                            eprintln!("Error reposting status: {}", e);
                        }
                    }
                    last_check_message_id = Some(latest.id);
                }
            }
        }
    }

    async fn update_status(&self, ctx: &Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let status_text = self.get_server_status().await;
        
        let mut message_id = self.last_message_id.write().await;
        
        if let Some(msg_id) = *message_id {
            let edit = EditMessage::new()
                .content(&status_text)
                .flags(MessageFlags::SUPPRESS_NOTIFICATIONS);
            match self.channel_id.edit_message(&ctx.http, msg_id, edit).await {
                Ok(_) => {}
                Err(_) => {
                    let builder = CreateMessage::new()
                        .content(&status_text)
                        .flags(MessageFlags::SUPPRESS_NOTIFICATIONS);
                    let new_msg = self.channel_id.send_message(&ctx.http, builder).await?;
                    *message_id = Some(new_msg.id);
                }
            }
        } else {
            let builder = CreateMessage::new()
                .content(&status_text)
                .flags(MessageFlags::SUPPRESS_NOTIFICATIONS);
            let msg = self.channel_id.send_message(&ctx.http, builder).await?;
            *message_id = Some(msg.id);
        }
        
        Ok(())
    }

    async fn repost_status(&self, ctx: &Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let status_text = self.get_server_status().await;
        
        let mut message_id = self.last_message_id.write().await;
        if let Some(msg_id) = *message_id {
            let _ = self.channel_id.delete_message(&ctx.http, msg_id).await;
        }
        
        let builder = CreateMessage::new()
            .content(&status_text)
            .flags(MessageFlags::SUPPRESS_NOTIFICATIONS);
        let new_msg = self.channel_id.send_message(&ctx.http, builder).await?;
        *message_id = Some(new_msg.id);
        
        Ok(())
    }

    async fn get_server_status(&self) -> String {
        let now = Local::now();
        let timestamp = now.format("%H:%M").to_string();
        let lang = env::var("LANGUAGE").unwrap_or_else(|_| "JP".to_string());
        let is_en = lang.to_uppercase() == "EN";
        
        match self.ping_server().await {
            Ok(info) => {
                if is_en {
                    format!(
                        "**Minecraft Bedrock Server Status**\n\
                        Server IP: `{}`\n\
                        Port: `{}`\n\
                        Status: 🟢 Online\n\
                        Players: {}/{}\n\
                        Last Updated: {}",
                        self.server_ip, self.server_port,
                        info.online_players, info.max_players,
                        timestamp
                    )
                } else {
                    format!(
                        "**Minecraft Bedrock サーバー状態**\n\
                        サーバーIP: `{}`\n\
                        ポート: `{}`\n\
                        サーバー状態: 🟢 オンライン\n\
                        プレイヤー数: {}/{}\n\
                        最終更新: {}",
                        self.server_ip, self.server_port,
                        info.online_players, info.max_players,
                        timestamp
                    )
                }
            }
            Err(_) => {
                if is_en {
                    format!(
                        "**Minecraft Bedrock Server Status**\n\
                        Server IP: `{}`\n\
                        Port: `{}`\n\
                        Status: 🔴 Offline\n\
                        Last Updated: {}",
                        self.server_ip, self.server_port,
                        timestamp
                    )
                } else {
                    format!(
                        "**Minecraft Bedrock サーバー状態**\n\
                        サーバーIP: `{}`\n\
                        ポート: `{}`\n\
                        サーバー状態: 🔴 オフライン\n\
                        最終更新: {}",
                        self.server_ip, self.server_port,
                        timestamp
                    )
                }
            }
        }
    }

    async fn ping_server(&self) -> Result<ServerInfo, Box<dyn std::error::Error + Send + Sync>> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(format!("127.0.0.1:{}", self.server_port)).await?;

        let mut packet = Vec::with_capacity(33);
        packet.push(0x01);
        
        let valid_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        packet.write_u64::<BigEndian>(valid_time)?;
        
        let magic = [
            0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78
        ];
        packet.extend_from_slice(&magic);
        
        packet.write_u64::<BigEndian>(rand::random())?;

        socket.send(&packet).await?;

        let mut buf = [0u8; 1024];
        let result = timeout(Duration::from_secs(2), socket.recv(&mut buf)).await?;
        let len = result?;

        if len > 0 && buf[0] == 0x1c {
            let mut cursor = Cursor::new(&buf[1..len]);
            if cursor.position() + 32 <= len as u64 {
                cursor.set_position(cursor.position() + 32);
                
                let magic_idx = buf.windows(16).position(|window| window == magic);
                if let Some(idx) = magic_idx {
                    let string_start = idx + 16 + 2;
                    if string_start < len {
                        let data_str = String::from_utf8_lossy(&buf[string_start..len]);
                        let parts: Vec<&str> = data_str.split(';').collect();
                        
                        if parts.len() >= 6 {
                            if let (Ok(online), Ok(max)) = (parts[4].parse::<i32>(), parts[5].parse::<i32>()) {
                                return Ok(ServerInfo {
                                    online_players: online,
                                    max_players: max,
                                });
                            }
                        }
                    }
                }
            }
        }

        Err("Failed to ping server".into())
    }
}

#[derive(Debug, Clone)]
struct ServerInfo {
    online_players: i32,
    max_players: i32,
}
