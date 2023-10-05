use std::env;
use std::time::Duration;

use dotenv::dotenv;
use serenity::async_trait;
use serenity::builder::CreateButton;
use serenity::client::{Context, EventHandler};
use serenity::futures::StreamExt;
use serenity::model::application::component::ButtonStyle;
use serenity::model::prelude::*;
use serenity::prelude::*;

mod storage;

fn sound_button(name: &str, emoji: ReactionType) -> CreateButton {
    let mut b = CreateButton::default();
    b.custom_id(name);
    // To add an emoji to buttons, use .emoji(). The method accepts anything ReactionType or
    // anything that can be converted to it. For a list of that, search Trait Implementations in the
    // docs for From<...>.
    b.emoji(emoji);
    b.label(name);
    b.style(ButtonStyle::Primary);
    b
}

struct Handler {
    db: storage::Data,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content != "!director" {
            return;
        }

        // Ask the user for its favorite director
        let m = msg
            .author
            .dm(&ctx, |m| {
                m.content("Please select your favorite director")
                    .components(|c| {
                        c.create_action_row(|row| {
                            // An action row can only contain one select menu!
                            row.create_select_menu(|menu| {
                                menu.custom_id("director_select");
                                menu.placeholder("No director selected");
                                menu.options(|f| {
                                    f.create_option(|o| {
                                        o.label("🎬 Steven Spielberg").value("Steven Spielberg")
                                    });
                                    f.create_option(|o| {
                                        o.label("🎬 Stanley Kubrick").value("Stanley Kubrick")
                                    });
                                    f.create_option(|o| {
                                        o.label("🎬 Martin Scorsese").value("Martin Scorsese")
                                    });
                                    f.create_option(|o| {
                                        o.label("🎬 Alfred Hitchcock").value("Alfred Hitchcock")
                                    });
                                    f.create_option(|o| {
                                        o.label("🎬 Quentin Tarantino").value("Quentin Tarantino")
                                    })
                                })
                            })
                        })
                    })
            })
            .await
            .unwrap();

        // Wait for the user to make a selection
        // This uses a collector to wait for an incoming event without needing to listen for it
        // manually in the EventHandler.
        let interaction = match m
            .await_component_interaction(&ctx)
            .timeout(Duration::from_secs(60 * 3))
            .await
        {
            Some(x) => x,
            None => {
                m.reply(&ctx, "Sorry, I can't sit around waiting all day.")
                    .await
                    .unwrap();
                return;
            }
        };

        // data.values contains the selected value from each select menus. We only have one menu,
        // so we retrieve the first
        let director = &interaction.data.values[0];

        // Acknowledge the interaction and edit the message
        interaction
            .create_interaction_response(&ctx, |r| {
                r.kind(InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|d| {
                        d.content(format!(
                            "You chose: **{}**\nNow choose a command!",
                            director
                        ))
                        .components(|c| {
                            c.create_action_row(|r| {
                                // add_XXX methods are an alternative to create_XXX methods
                                r.add_button(sound_button("action", "📣".parse().unwrap()));
                                r.add_button(sound_button("cut", "📣".parse().unwrap()));
                                r.add_button(sound_button("print it", "📣".parse().unwrap()));
                                r.add_button(sound_button("another take", "📣".parse().unwrap()));
                                r.add_button(sound_button("that's a wrap", "📣".parse().unwrap()))
                            })
                        })
                    })
            })
            .await
            .unwrap();

        // Wait for multiple interactions
        let mut interaction_stream = m
            .await_component_interactions(&ctx)
            .timeout(Duration::from_secs(60 * 3))
            .build();

        while let Some(interaction) = interaction_stream.next().await {
            let action = &interaction.data.custom_id;
            // Acknowledge the interaction and send a reply
            interaction
                .create_interaction_response(&ctx, |r| {
                    // This time we dont edit the message but reply to it
                    r.kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|d| {
                            // Make the message hidden for other users by setting `ephemeral(true)`.
                            d.ephemeral(true)
                                .content(format!("**{}** yells __{}__!", director, action))
                        })
                })
                .await
                .unwrap();
        }

        // Delete the orig message or there will be dangling components (components that still
        // exist, but no collector is running so any user who presses them sees an error)
        m.delete(&ctx).await.unwrap()
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let db_file = env::var("DB_FILE").expect("Expected a db file in the environment.");

    let db = storage::init(&db_file)
        .await
        .expect("Could not initialize db.");
    let handler = Handler { db };

    // Build our client.
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .await
        .expect("Error creating client");

    // Finally, start a single shard, and start listening to events.
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
