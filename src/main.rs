use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use futures::StreamExt;
use gtk::prelude::{BoxExt, ButtonExt, EditableExt, EntryExt, GtkWindowExt, OrientableExt};
use matrix_sdk::{config::SyncSettings, reqwest::Url, ruma::RoomId, Client};
use relm4::{
    factory::{FactoryPrototype, FactoryVecDeque},
    gtk, send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets,
};

mod secrecy;
use secrecy::SecretString;

type SpaceReference = Arc<Mutex<Space>>;

/// Relevant data for a user account.
struct Account {
    pub client: matrix_sdk::Client,
    pub rooms: HashMap<Box<RoomId>, Room>,
    pub spaces: Vec<SpaceReference>,
}

/// Data related to a single matrix room.
struct Room {
    pub sdk_room: matrix_sdk::room::Room,
}

/// Graph of spaces, referencing the [[Account.rooms]] collection.
///
/// Invariant: The graph of spaces forms a DAG.
struct Space {
    room_id: Box<RoomId>,
    contained_rooms: Vec<Box<RoomId>>,
    children: Vec<SpaceReference>,
}

impl Space {
    /// Iterate over all direct children of this space.
    pub fn children(&self) -> impl Iterator<Item = SpaceReference> + '_ {
        self.children.iter().cloned()
    }

    /// Traverse the subgraph reachable from this space.
    ///
    /// Spaces that are reachable via multiple parent spaces are visited multiple times.
    pub fn traverse<Action: FnMut(&Space)>(self, mut action: Action) {
        let mut stack: Vec<SpaceReference> = self.children().collect();

        action(&self);

        while !stack.is_empty() {
            // unwrap is safe, as we exit the loop if the stack is empty
            let current = stack.pop().unwrap();

            let current = current.lock().unwrap();

            for c in current.children() {
                stack.push(c);
            }
            action(&current);
        }
    }
}

#[tracker::track]
struct AppModel {
    login_user_message: String,
}

impl AppModel {
    fn new() -> Self {
        Self {
            login_user_message: String::new(),
            tracker: 0,
        }
    }
}

#[derive(Debug)]
enum AppMsg {
    Login {
        username: String,
        password: SecretString,
        homeserver: String,
    },
    LoginResult {
        result: Result<(), matrix_sdk::Error>,
    },
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = ();
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: AppMsg, _components: &(), sender: Sender<AppMsg>) -> bool {
        // reset tracked changes
        self.reset();

        match msg {
            AppMsg::Login {
                username,
                password,
                homeserver,
            } => {
                use secrecy::ExposeSecret;
                println!("Trying to log in @{}:{}", username, homeserver);
                tokio::spawn(async move {
                    let result = login(homeserver, &username, password.expose_secret()).await;
                    send!(sender, AppMsg::LoginResult { result });
                });
            }
            AppMsg::LoginResult { result } => match result {
                Ok(_) => self.set_login_user_message("Login successful!".to_string()),
                Err(e) => {
                    use matrix_sdk::{
                        ruma::api::error::FromHttpResponseError, Error::Http, HttpError,
                        RumaApiError,
                    };
                    let message = match e {
                        Http(HttpError::Api(FromHttpResponseError::<RumaApiError>::Server(
                            code,
                        ))) => format!("Computer said no: {}", code),
                        _ => e.to_string(),
                    };
                    self.set_login_user_message(message);
                }
            },
        }
        true
    }
}

#[relm4::widget]
impl Widgets<AppModel, ()> for AppWidgets {
    view! {
        gtk::ApplicationWindow {
            set_title: Some("maruc"),
            set_default_width: 300,
            set_default_height: 100,
            set_child = Some(&gtk::Box) {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,

                append: homeserver = &gtk::Entry {
                    set_placeholder_text: Some("home server")
                },

                append: username = &gtk::Entry {
                    set_placeholder_text: Some("username")
                },

                append: password = &gtk::PasswordEntry{
                    set_placeholder_text: Some("password")
                },

                append = &gtk::Button {
                    set_label: "Login",
                    connect_clicked(sender, username, password, homeserver) => move |_| {
                        send!(sender, AppMsg::Login{
                            username: username.text().to_string(),
                            password: SecretString::new(password.text().to_string()),
                            homeserver: homeserver.text().to_string()
                        });
                        username.set_text("");
                        password.set_text("");
                        homeserver.set_text("");
                    },
                },

                append = &gtk::Label {
                    set_text: track!(model.changed(AppModel::login_user_message()), &model.login_user_message)
                },
            },
        }
    }
}

async fn login(
    homeserver_url: String,
    username: &str,
    password: &str,
) -> Result<(), matrix_sdk::Error> {
    let homeserver_url = Url::parse(&homeserver_url).expect("Couldn't parse the homeserver URL");
    let client = Client::new(homeserver_url).await.unwrap();

    client
        .login(username, password, None, Some("maruc"))
        .await?;

    tokio::spawn(async move {
        let mut sync_stream = Box::pin(client.sync_stream(SyncSettings::default()).await);
        while let Some(Ok(response)) = sync_stream.next().await {
            for room in response.rooms.join.values() {
                for e in &room.timeline.events {
                    if let Ok(event) = e.event.deserialize() {
                        println!("Received event {:?}", event);
                    }
                }
            }
        }
    });

    Ok(())
}

#[tokio::main]
async fn main() {
    tokio::task::spawn_blocking(|| {
        gtk::init().expect("Failed to initialize GTK!");
        let model = AppModel::new();
        let app = RelmApp::new(model);
        app.run();
    })
    .await
    .unwrap();
}
