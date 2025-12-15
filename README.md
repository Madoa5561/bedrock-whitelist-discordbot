# Minecraft Bedrock Allowlist & Status Bot

Minecraft Bedrock (統合版) サーバーのホワイトリスト管理とステータス表示を行うDiscordボットです。

## 機能

- **/server コマンド**: ユーザーが自分でMinecraftのゲーマータグを入力し、サーバーのAllowlist（許可リスト）に追加できます。
- **ステータス監視**: 指定したチャンネルにサーバーの状態（オンライン/オフライン、参加人数）をリアルタイムで表示します。30秒ごとに更新されます。
- **Unconnected Ping 対応**: RakNetプロトコルを使用して、ゲーム内と同様の正確なステータスを取得します。
- **多言語対応**: 環境変数で日本語（JP）と英語（EN）を切り替え可能です。

## 必要要件

- Rust (最新の安定版)
- Discord Bot Token

## インストールとセットアップ

1. **リポジトリのクローン**
   ```bash
   git clone <repository-url>
   cd Allowbot
   ```

2. **環境変数の設定**
   `.env` ファイルを作成し、以下の内容を設定してください：

   ```env
   # Discordボットのトークン
   DISCORD_TOKEN=your_token_here
   
   SERVER_PATH=../bedrock_server.exe
   # Can be a directory (../) or full path to exe (../bedrock_server.exe)
   
   # ステータスを表示するチャンネルID
   STATUS_CHANNEL_ID=123456789012345678
   
   # Allowlistファイルのパス（サーバーのallowlist.jsonを指定）
   ALLOWLIST_PATH=../allowlist.json

   # Discord上に実際に表示されるIP
   SERVER_IP=mc.example.org
   
   # 監視対象のサーバーIPとポート（デフォルトは127.0.0.1:19132）
   INTERNAL_IP=127.0.0.1
   SERVER_PORT=19132

   # 言語設定 (JP または EN)
   LANGUAGE=JP
   ```

3. **ビルド**
   ```bash
   cargo build --release
   ```

4. **実行**
   ```bash
   # 通常実行
   cargo run --release
   ```

## ディレクトリ構造

- `src/`: ソースコード
  - `main.rs`: エントリーポイント、環境変数の読み込み
  - `commands.rs`: スラッシュコマンドとModalの処理
  - `status.rs`: サーバーステータスの監視とPing処理（UDP/RakNet）
  - `allowlist.rs`: allowlist.jsonの読み書き管理

## 技術的詳細

- **Serenity**: Discord APIとの対話に使用
- **Tokio**: 非同期ランタイム
- **UDP Socket**: BedrockサーバーへのPing送信に使用（自前実装のUnconnected Ping）

## ライセンス

[MIT License](LICENSE)


