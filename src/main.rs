use chrono::{format::Locale, Date, Local, NaiveDate, TimeZone};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::{env::var, error::Error};
use teloxide::{
    dispatching::{dialogue::InMemStorage, UpdateFilterExt},
    dptree::{case, deps, entry},
    payloads::SendMessageSetters,
    prelude::*,
    types::{
        BotCommandScope, InlineKeyboardButton, InlineKeyboardButtonKind, InlineKeyboardMarkup,
    },
    utils::command::BotCommands,
};

mod db;
mod domain;

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;
type MyBot = AutoSend<Bot>;

#[derive(BotCommands, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum CustomerCommand {
    #[command(description = "Display this text")]
    Help,
    #[command(description = "Start")]
    Start,
}

#[derive(Debug, Clone)]
enum State {
    Start,
    ReceiveMenu(Vec<domain::MenuItemId>),
    ReceiveDate { order: Vec<domain::MenuItemId> },
    ReceiveDateAfter { order: Vec<domain::MenuItemId> },
}

impl Default for State {
    fn default() -> Self {
        State::Start
    }
}

type AppointmentFlow = Dialogue<State, InMemStorage<State>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CallbackData {
    Menu { menu_item_id: domain::MenuItemId },
    Date { date: NaiveDate },
}

impl TryFrom<String> for CallbackData {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let raw_data = bs58::decode(value)
            .into_vec()
            .map_err(anyhow::Error::from)?;
        bincode::deserialize(&raw_data).map_err(anyhow::Error::from)
    }
}

impl ToString for CallbackData {
    fn to_string(&self) -> String {
        let raw_data = bincode::serialize(self).unwrap();
        bs58::encode(raw_data).into_string()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::init();
    log::info!("Connecting to database...");

    let pool = sqlx::PgPool::connect(&var("DATABASE_URL").expect("No DATABASE_URL is set")).await?;
    log::info!("Applying database migrations...");
    db::migration::run(&pool).await?;

    let menu_repo = domain::MenuItemRepo::Postgres(pool.clone());

    log::info!("Starting bot...");
    let bot = Bot::new(&var("BOT_TOKEN").expect("No BOT_TOKEN is set")).auto_send();

    bot.set_my_commands(CustomerCommand::bot_commands())
        .scope(BotCommandScope::AllPrivateChats)
        .language_code("ru")
        .await?;

    let handler = entry()
        .branch(Update::filter_my_chat_member().endpoint(bot_chat_membership))
        //.branch(Update::filter_chat_member().endpoint(chat_member_update_hanlder))
        .branch(
            Update::filter_message().branch(
                entry()
                    .filter_command::<CustomerCommand>()
                    .branch(case!(CustomerCommand::Start).endpoint(customer_start))
                    .branch(case!(CustomerCommand::Help).endpoint(customer_help)),
            ),
        )
        .branch(
            Update::filter_callback_query()
                .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
                .endpoint(handle_callback_query),
        );

    let state_storage = InMemStorage::<State>::new();

    Dispatcher::builder(bot, handler)
        .dependencies(deps!(menu_repo, state_storage))
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;

    Ok(())
}

async fn handle_callback_query(
    q: CallbackQuery,
    bot: MyBot,
    flow: AppointmentFlow,
    menu_repo: domain::MenuItemRepo,
) -> HandlerResult {
    if let Some(data) = q.data.clone() {
        let state = flow.get_or_default().await?;
        match CallbackData::try_from(data) {
            Ok(callback_data) => match callback_data {
                CallbackData::Menu { menu_item_id } => match state {
                    State::Start => {
                        menu_item_selected(q, bot, vec![], menu_item_id, menu_repo, flow).await?;
                    }
                    State::ReceiveMenu(items) => {
                        menu_item_selected(q, bot, items, menu_item_id, menu_repo, flow).await?;
                    }
                    _ => {
                        log::warn!("Illegal state {:?}", state);
                    }
                },
                CallbackData::Date { date } => {
                    appointment_date_selected(q, bot, date, flow).await?;
                }
            },
            Err(e) => {
                log::error!("Failed to parse callback data: {}", e);

                if let Some(msg) = q.message {
                    bot.send_message(msg.chat.id, "Error processing response")
                        .await?;
                }
            }
        }
    } else {
        log::warn!("Received callback query with empty data");
    }
    Ok(())
}

async fn appointment_date_selected(
    q: CallbackQuery,
    bot: MyBot,
    date: NaiveDate,
    flow: AppointmentFlow,
) -> HandlerResult {
    Ok(())
}

async fn menu_item_selected(
    q: CallbackQuery,
    bot: MyBot,
    menu_items: Vec<domain::MenuItemId>,
    menu_item_id: domain::MenuItemId,
    menu_repo: domain::MenuItemRepo,
    flow: AppointmentFlow,
) -> HandlerResult {
    let Message { id, chat, .. } = q.message.unwrap();

    let mut new_items = Vec::from_iter(menu_items);
    new_items.push(menu_item_id);

    let next_items = menu_repo
        .find_by_org(domain::OrgId(0), Some(menu_item_id))
        .await?;

    if !next_items.is_empty() {
        flow.update(State::ReceiveMenu(new_items)).await?;

        bot.edit_message_reply_markup(chat.id, id)
            .reply_markup(make_menu(&next_items))
            .await?;
    } else {
        flow.update(State::ReceiveDate { order: new_items }).await?;

        let text = format!("Выберите удобное время:");
        bot.edit_message_text(chat.id, id, text).await?;

        bot.edit_message_reply_markup(chat.id, id)
            .reply_markup(date_picker(Local::now().date(), 0))
            .await?;
    }

    Ok(())
}

fn date_picker(start_date: Date<Local>, offset: usize) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let start_date = start_date.naive_local();

    for date in start_date.iter_days().skip(offset + 1).take(5) {
        let diff = date - start_date;
        let format = match diff.num_days() {
            1 => "Завтра, %e %B",
            2 => "Послезавтра, %e %B",
            _ => "%A, %e %B",
        }
        .to_string();
        let text = Local
            .from_local_date(&date)
            .unwrap()
            .format_localized(&format, Locale::ru_RU)
            .to_string();
        let data = InlineKeyboardButtonKind::CallbackData(CallbackData::Date { date }.to_string());
        let button = InlineKeyboardButton::new(text, data);
        keyboard.push(vec![button]);
    }

    InlineKeyboardMarkup::new(keyboard)
}

async fn customer_start(
    msg: Message,
    bot: MyBot,
    menu_repo: domain::MenuItemRepo,
) -> HandlerResult {
    let items = menu_repo.find_by_org(domain::OrgId(0), None).await?;
    let menu = make_menu(&items);
    bot.send_message(msg.chat.id, "Выберите услугу:")
        .reply_markup(menu)
        .await?;
    Ok(())
}

async fn customer_help(msg: Message, bot: MyBot) -> HandlerResult {
    bot.send_message(msg.chat.id, CustomerCommand::descriptions().to_string())
        .await?;
    Ok(())
}

async fn bot_chat_membership(
    event: ChatMemberUpdated,
    _bot: AutoSend<Bot>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    log::info!("Chat member updated: {:?}", event);
    Ok(())
}

/// Creates a keyboard made by buttons in a big column.
fn make_menu(items: &[domain::MenuItem]) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    for item in items {
        let price = match item.price {
            domain::IntervalValue::Value(v) => format!("{}", v),
            domain::IntervalValue::Interval(min, max) => format!("{}-{}", min, max),
        };
        let duration = match item.duration {
            domain::IntervalValue::Value(v) => format!("{}", v.num_minutes()),
            domain::IntervalValue::Interval(min, max) => {
                format!("{}-{}", min.num_minutes(), max.num_minutes())
            }
        };

        let text = format!("{}: ({}мин) {}руб", item.title, duration, price);
        let data = CallbackData::Menu {
            menu_item_id: item.id,
        };

        keyboard.push(vec![InlineKeyboardButton::new(
            text,
            InlineKeyboardButtonKind::CallbackData(data.to_string()),
        )]);
    }

    InlineKeyboardMarkup::new(keyboard)
}
