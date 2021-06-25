#[macro_use]
extern crate lazy_static;

use std::{
    collections::HashSet,
    env,
    sync::Arc,
};
use linked_hash_map::LinkedHashMap;

use serde::{Deserialize, Serialize};
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::{
        standard::macros::group,
        StandardFramework,
    },
    http::Http,
    model::{event::ResumedEvent, gateway::Ready},
    prelude::*,
};
use serenity::framework::standard::{
    CommandResult,
    macros::hook
};
use serenity::utils::Colour;
use serenity::model::channel::Message;
use serenity::model::gateway::Activity;
use tracing::{error, info};
use tracing_subscriber::{
    EnvFilter,
    FmtSubscriber,
};

use commands::{
    user::*
};

mod commands;

#[derive(Debug, Serialize, Deserialize)]
struct Packet {
    id: String,
    name: String,
}

#[derive(Debug)]
struct PacketData {
    server_bound: LinkedHashMap<String, String>,
    client_bound: LinkedHashMap<String, String>
}

#[derive(Debug, Serialize, Deserialize)]
struct ReceivingPacketData {
    #[serde(rename = "serverBound")]
    server_bound: Vec<Packet>,
    #[serde(rename = "clientBound")]
    client_bound: Vec<Packet>
}

impl PacketData {
    fn new() -> PacketData {
        PacketData {
            server_bound: LinkedHashMap::new(),
            client_bound: LinkedHashMap::new()
        }
    }
}

lazy_static! {
    static ref PACKET_DATA: std::sync::Mutex<PacketData> = std::sync::Mutex::new(PacketData::new());
}

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

#[hook]
async fn after(ctx: &Context, msg: &Message, _cmd: &str, result: CommandResult) {
    if let Err(error) = result {
        msg.channel_id
            .send_message(&ctx, |m| {
                m.reference_message(msg).allowed_mentions(|f| {
                    f.replied_user(true)
                        .parse(serenity::builder::ParseValue::Everyone)
                        .parse(serenity::builder::ParseValue::Users)
                        .parse(serenity::builder::ParseValue::Roles)
                });
                m.embed(|e| {
                    e.description(
                        "**An error occurred:**\n".to_owned() +
                        "```rs\n" +
                        &*error.to_string() +
                        "\n```"
                    );
                    e.color(Colour::RED)
                })
            }).await.unwrap();
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}#{}", ready.user.name, ready.user.discriminator);
        ctx.set_activity(Activity::watching("Muck")).await;
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[group]
#[commands(i2p, list)]
struct General;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    info!("Fetching data.");

    let resp: ReceivingPacketData = reqwest::get(
        "https://gist.githubusercontent.com/Lucaskyy/26284f56765dc05201f5ea31cbfaf548/raw/487b1857901e3d63d004f8000b55e99d39c7bad8/data.json"
    )
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    info!("Received data, putting into memory.");

    for t in resp.client_bound.iter() {
        PACKET_DATA.lock().unwrap().client_bound.insert((*t.id).to_string(), (*t.name).to_string());
    }
    for t in resp.server_bound.iter() {
        PACKET_DATA.lock().unwrap().server_bound.insert((*t.id).to_string(), (*t.name).to_string());
    }

    info!("Fetched and loaded packet data into memory!");

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");
    let prefix = env::var("PREFIX")
        .expect("Expected a prefix in the environment");

    let http = Http::new_with_token(&token);

    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        },
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| c
            .owners(owners)
            .prefix(&*prefix))
        .group(&GENERAL_GROUP)
        .after(after);

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Error creating client!");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Could not register handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}