use crate::ShardManagerContainer;
use serenity::model::channel::Message;
use serenity::prelude::*; // We need to import our TypeMapKey from main.rs

// This is the function that will be called when the `!ping` command is used.
pub async fn run(ctx: &Context, msg: &Message) {
    let data = ctx.data.read().await;
    if let Some(shard_manager) = data.get::<ShardManagerContainer>() {
        let runners = shard_manager.runners.lock().await;
        if let Some(runner) = runners.get(&ctx.shard_id) {
            let latency = runner.latency.map_or_else(
                || "N/A".to_string(),
                |latency| format!("{:.2} ms", latency.as_millis()),
            );
            let response = format!("Pong! Heartbeat Latency: `{}`", latency);
            if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                println!("Error sending ping response: {:?}", why);
            }
        }
    }
}
