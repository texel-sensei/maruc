use druid::im::Vector;
use druid::widget::{Button, Flex, Label, List};
use druid::{AppLauncher, Data, Lens, PlatformError, Widget, WidgetExt, WindowDesc};

use matrix_sdk::reqwest::Url;
use matrix_sdk::ruma::events::SyncMessageEvent;
use matrix_sdk::ruma::events::room::message::MessageEventContent;
use matrix_sdk::{self, Client, SyncSettings};
use matrix_sdk::ruma::{user_id, events::room::message::MessageEvent};

// for reference: https://github.com/futurepaul/druid-todo-tutorial
//
// other chat software in druid: https://github.com/loipesmas/accord

#[derive(Clone, Data, Lens)]
struct Message {
    pub text: String,
    pub user: i32,
}

impl Message {
    pub fn new<S: Into<String>>(text: S, user: i32) -> Self {
        Message {
            text: text.into(),
            user,
        }
    }
}

#[derive(Clone, Data, Lens)]
struct AppData {
    pub history: Vector<Message>,
}

impl AppData {
    pub fn new() -> Self {
        AppData {
            history: Vector::new(),
        }
    }
}

fn message() -> impl Widget<Message> {
    Label::new("test")
}

fn build_ui() -> impl Widget<AppData> {
    Flex::column()
        .with_child(
            Button::new("Add").on_click(|_ctx, data: &mut AppData, _env| {
                data.history.push_back(Message::new("test", 17))
            }),
        )
        .with_child(List::new(message).lens(AppData::history))
}

async fn on_event(event: SyncMessageEvent<MessageEventContent>) {
    dbg!(event);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    /*
    let blocking_task = tokio::task::spawn_blocking( || {
        let state = AppData::new();
        AppLauncher::with_window(WindowDesc::new(build_ui)).launch(state)?;
        Ok(())
    });
    */

    //blocking_task.await.unwrap()

    let client = Client::new(Url::parse("https://matrix.org")?)?;

    client.register_event_handler(on_event).await;
    client.login("user", "pw", None, Some("at.texel.maruc")).await?;

    client.sync(SyncSettings::new()).await;

    Ok(())
}
