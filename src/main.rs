use std::collections::HashMap;

use gtk::prelude::{BoxExt, ButtonExt, EditableExt, EntryExt, GtkWindowExt, OrientableExt};
use matrix_sdk::{config::SyncSettings, reqwest::Url, ruma::OwnedRoomId, Client};
use relm4::{gtk, send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};

mod secrecy;
use secrecy::SecretString;
mod spacegraph;
use spacegraph::*;

/// Relevant data for a user account.
struct Account {
    pub client: matrix_sdk::Client,
    pub rooms: HashMap<OwnedRoomId, Room>,
    pub spaces: Vec<SpaceReference>,
}

/// Data related to a single matrix room.
struct Room {
    pub sdk_room: matrix_sdk::room::Room,
}

impl Room {
    fn new(sdk_room: matrix_sdk::room::Room) -> Self {
        Self { sdk_room }
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

enum AppMsg {
    Login {
        username: String,
        password: SecretString,
        homeserver: String,
    },
    LoginResult {
        result: Result<Account, matrix_sdk::Error>,
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
) -> Result<Account, matrix_sdk::Error> {
    let homeserver_url = Url::parse(&homeserver_url).expect("Couldn't parse the homeserver URL");
    let client = Client::new(homeserver_url).await.unwrap();

    client
        .login(username, password, None, Some("maruc"))
        .await?;

    // TODO(texel, 2022-07-17): replace with background worker (WK-31)
    client.sync_once(SyncSettings::new()).await?;

    let rooms = client
        .rooms()
        .iter()
        .map(|r| (r.room_id().to_owned(), Room::new(r.clone())))
        .collect();

    // TODO(texel, 2022-07-17): This ignores the space hierarchy and assumes all spaces to be roots
    let spaces = client
        .joined_rooms()
        .iter()
        .filter(|r| r.is_space())
        .map(|r| Space::new(r.room_id().to_owned()))
        .collect();

    Ok(Account {
        client,
        rooms,
        spaces,
    })
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
