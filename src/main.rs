use dotenv::dotenv;
use std::{env::var, error::Error};
use teloxide::{
    dispatching::UpdateFilterExt,
    payloads::SendMessageSetters,
    prelude::*,
    types::{
        BotCommandScope, ChatMemberKind, InlineKeyboardButton, InlineKeyboardMarkup,
        InlineQueryResultArticle, InputMessageContent, InputMessageContentText, MessageKind,
    },
    utils::command::BotCommands,
};

mod db;
mod domain;

#[derive(BotCommands)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "Display this text")]
    Help,
    #[command(description = "Start")]
    Start,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::init();
    log::info!("Starting buttons bot...");

    let pool = sqlx::PgPool::connect(&var("DATABASE_URL").expect("No DATABASE_URL is set")).await?;
    db::migration::run(&pool).await?;

    let bot = Bot::new(&var("BOT_TOKEN").expect("No BOT_TOKEN is set")).auto_send();

    bot.set_my_commands(Command::bot_commands())
        .scope(BotCommandScope::AllPrivateChats)
        .await?;
    //bot.set_chat_menu_button().chat_id().menu_button().await?;

    let handler = dptree::entry()
        .branch(Update::filter_my_chat_member().endpoint(bot_chat_membership))
        //.branch(Update::filter_chat_member().endpoint(chat_member_update_hanlder))
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;

    Ok(())
}

async fn bot_chat_membership(
    event: ChatMemberUpdated,
    bot: AutoSend<Bot>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    log::info!("Chat member updated: {:?}", event);
    Ok(())
}

/// Creates a keyboard made by buttons in a big column.
fn make_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    InlineKeyboardMarkup::new(keyboard)
}

/// Parse the text wrote on Telegram and check if that text is a valid command
/// or not, then match the command. If the command is `/start` it writes a
/// markup with the `InlineKeyboardMarkup`.
async fn message_handler(
    m: Message,
    bot: AutoSend<Bot>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(text) = m.text() {
        match BotCommands::parse(text, "apt111bot") {
            Ok(Command::Help) => {
                // Just send the description of all commands.
                bot.send_message(m.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Ok(Command::Start) => {
                // Create a list of buttons and send them.
                let keyboard = make_keyboard();
                bot.send_message(m.chat.id, "Debian versions:")
                    .reply_markup(keyboard)
                    .await?;
            }

            Err(_) => {
                bot.send_message(m.chat.id, "Command not found!").await?;
            }
        }
    }

    Ok(())
}

/// When it receives a callback from a button it edits the message with all
/// those buttons writing a text with the selected Debian version.
///
/// **IMPORTANT**: do not send privacy-sensitive data this way!!!
/// Anyone can read data stored in the callback button.
async fn callback_handler(
    q: CallbackQuery,
    bot: AutoSend<Bot>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(version) = q.data {
        let text = format!("You chose: {version}");

        let url = url::Url::parse("https://t.me/app111bot?start")?;
        match q.message {
            Some(Message { id, chat, .. }) => {
                bot.answer_callback_query(q.id).url(url).await?;
                //bot.edit_message_text(chat.id, id, text).await?;
                bot.send_message(q.from.id, text).await?;
                bot.delete_message(chat.id, id).await?;
            }
            None => {
                if let Some(id) = q.inline_message_id {
                    bot.edit_message_text_inline(id, text).await?;
                };
            }
        };

        log::info!("You chose: {}", version);
    }

    Ok(())
}
