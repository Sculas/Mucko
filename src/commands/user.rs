use serenity::framework::standard::{Args, CommandResult, macros::command, CommandError};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Colour;
use crate::PACKET_DATA;

#[command]
pub async fn i2p(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let id = args.single::<String>().or_else(|_e| Err("Usage: ~i2p <id> <bound>"))?;
    if !is_string_numeric(id.clone()) {
        return Err(CommandError::from("Invalid ID, must be a number!"))
    }

    let bound = args.single::<String>().or_else(|_e| Err("Usage: ~i2p <id> <bound>"))?.to_lowercase();
    let mut c2s = false;

    if bound == "c2s" {
        c2s = true;
    } else if bound != "s2c" {
        return Err(CommandError::from("Invalid bound, please use \"c2s\" OR \"s2c\"!"))
    }

    let packet_name = {
        let lock = PACKET_DATA.lock().unwrap();
        if c2s {
            lock.client_bound.get(&*id).cloned().unwrap_or_else(|| "N/A".to_string())
        } else {
            lock.server_bound.get(&*id).cloned().unwrap_or_else(|| "N/A".to_string())
        }
    };

    if packet_name == "N/A" {
        return Err(CommandError::from("No such packet!"))
    }

    msg.channel_id
        .send_message(&ctx, |m| {
            m.reference_message(msg).allowed_mentions(|f| {
                f.replied_user(true)
                    .parse(serenity::builder::ParseValue::Everyone)
                    .parse(serenity::builder::ParseValue::Users)
                    .parse(serenity::builder::ParseValue::Roles)
            });
            m.embed(|e| {
                e.field("Packet ID", "`".to_owned() + &id + "`", false);
                e.field("Packet Name", "`".to_owned() + &packet_name + "`", false);
                e.field("Packet Bound", "`".to_owned() + &bound.to_uppercase() + "`", false);
                e.color(Colour::BLURPLE)
            })
        }).await?;

    Ok(())
}

#[command]
pub async fn list(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .send_message(&ctx, |m| {
            m.reference_message(msg).allowed_mentions(|f| {
                f.replied_user(true)
                    .parse(serenity::builder::ParseValue::Everyone)
                    .parse(serenity::builder::ParseValue::Users)
                    .parse(serenity::builder::ParseValue::Roles)
            });
            m.embed(|e| {
                let mut data: String;
                data = "".to_string();

                data = {
                    let lock = PACKET_DATA.lock().unwrap();
                    data += "**C2S:**\n```";
                    for t in lock.client_bound.iter() {
                        data += &*format!("{:<5}=>    {1}\n", t.0, t.1);
                    }
                    data += "```";
                    data += "**S2C:**\n```";
                    for t in lock.server_bound.iter() {
                        data += &*format!("{:<5}=>    {1}\n", t.0, t.1);
                    }
                    data += "```";
                    data
                };

                e.description(data);
                e.color(Colour::BLURPLE)
            })
        }).await?;

    Ok(())
}

fn is_string_numeric(str: String) -> bool {
    for c in str.chars() {
        if !c.is_numeric() {
            return false;
        }
    }
    return true;
}