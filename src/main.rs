use druid::im::Vector;
use druid::widget::{Button, Flex, Label, List};
use druid::{AppLauncher, Data, Lens, PlatformError, Widget, WidgetExt, WindowDesc};

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

fn main() -> Result<(), PlatformError> {
    let state = AppData::new();
    AppLauncher::with_window(WindowDesc::new(build_ui)).launch(state)?;
    Ok(())
}
