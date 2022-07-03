use futures::future::join_all;
use std::collections::HashMap;

use gtk::prelude::{BoxExt, ButtonExt, EditableExt, EntryExt, GtkWindowExt, OrientableExt};
use matrix_sdk::{
    config::SyncSettings,
    reqwest::Url,
    ruma::{events::space::child::SyncSpaceChildEvent, serde::Raw, OwnedRoomId, RoomId},
    Client,
};
use relm4::{gtk, send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};

mod secrecy;
use secrecy::SecretString;
mod spacegraph;
use spacegraph::*;

/// Relevant data for a user account.
struct Account {
    #[allow(dead_code)]
    pub client: matrix_sdk::Client,
    #[allow(dead_code)]
    pub rooms: HashMap<OwnedRoomId, Room>,
    #[allow(dead_code)]
    pub spaces: Vec<SpaceReference>,
}

#[derive(Debug)]
enum RoomType {
    Space,
    DirectMessage,
    Room,
}

/// Data related to a single matrix room.
struct Room {
    pub sdk_room: matrix_sdk::room::Room,
    pub name: matrix_sdk::DisplayName,
    pub rtype: RoomType,
}

impl Room {
    async fn new(sdk_room: matrix_sdk::room::Room) -> Self {
        let name = sdk_room
            .display_name()
            .await
            .unwrap_or(matrix_sdk::DisplayName::Empty);
        let rtype = if sdk_room.is_space() {
            RoomType::Space
        } else if sdk_room.is_direct() {
            RoomType::DirectMessage
        } else {
            RoomType::Room
        };

        Self {
            sdk_room,
            name,
            rtype,
        }
    }
}

impl std::fmt::Debug for Room {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{} \"{}\" {:?}]",
            self.sdk_room.room_id(),
            self.name,
            self.rtype
        )
    }
}

impl std::fmt::Display for Room {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
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
                    let result = login(&homeserver, &username, password.expose_secret()).await;
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
    homeserver_url: &str,
    username: &str,
    password: &str,
) -> Result<Account, matrix_sdk::Error> {
    let homeserver_url = Url::parse(homeserver_url).expect("Couldn't parse the homeserver URL");
    let client = Client::new(homeserver_url).await.unwrap();

    client
        .login(username, password, None, Some("maruc"))
        .await?;

    // TODO(texel, 2022-07-17): replace with background worker (WK-31)
    client.sync_once(SyncSettings::new()).await?;

    let rooms = client
        .rooms()
        .into_iter()
        .map(|r| tokio::spawn(Room::new(r)));

    let rooms = join_all(rooms)
        .await
        .into_iter()
        .map(|r| r.expect("Task has paniced!"));

    let rooms: HashMap<OwnedRoomId, Room> = rooms
        .map(|r| (r.sdk_room.room_id().to_owned(), r))
        .collect();
    //r.room_id().to_owned()

    let spaces: Vec<_> = client
        .joined_rooms()
        .iter()
        .filter(|r| r.is_space())
        .map(|r| Space::new(r.room_id().to_owned()))
        .collect();

    let mut possible_roots: HashMap<&RoomId, SpaceReference> =
        spaces.iter().map(|s| (s.room_id(), s.clone())).collect();

    for current in &spaces {
        let id = current.room_id();
        let room = client.get_room(id);

        let room = match room {
            Some(r) => r,
            None => continue,
        };

        let children: Vec<Raw<SyncSpaceChildEvent>> = room.get_state_events_static().await?;

        for c in children {
            let child_id: Option<&RoomId> = c
                .get_field("state_key")
                .ok()
                .flatten()
                .and_then(|id: &str| id.try_into().ok());

            let child_id = match child_id {
                Some(id) => id,
                None => continue,
            };

            if rooms[child_id].sdk_room.is_space() {
                let space_ref = spaces
                    .iter()
                    .find(|s| s.room_id() == child_id)
                    .unwrap()
                    .clone();

                let result = current.add_child(space_ref);
                if result.is_err() {
                    eprintln!(
                        "Would build cycle between {child_id} and {}!",
                        current.room_id()
                    );
                    continue;
                }
                possible_roots.remove(child_id);
            } else {
                current.insert_room(child_id);
            }
        }
    }

    println!("{:?}", rooms);
    println!("{:?}", spaces);
    println!("{:?}", possible_roots);

    Ok(Account {
        client,
        rooms,
        spaces: possible_roots.into_values().collect(),
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
