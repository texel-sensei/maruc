use gtk::prelude::{BoxExt, ButtonExt, EditableExt, EntryExt, GtkWindowExt, OrientableExt};
use relm4::{gtk, send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};
use secrecy::SecretString;

struct AppModel {
    counter: u8,
}

impl AppModel {
    fn new() -> Self {
        Self { counter: 0 }
    }
}

enum AppMsg {
    Increment,
    Decrement,
    Login {
        username: String,
        password: SecretString,
    },
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = ();
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: AppMsg, _components: &(), _sender: Sender<AppMsg>) -> bool {
        match msg {
            AppMsg::Increment => {
                self.counter = self.counter.wrapping_add(1);
            }
            AppMsg::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
            }
            AppMsg::Login { username, password } => {
                use secrecy::ExposeSecret;
                println!(
                    "Username: {} Password: {}",
                    username,
                    password.expose_secret()
                );
            }
        }
        true
    }
}

#[relm4::widget]
impl Widgets<AppModel, ()> for AppWidgets {
    view! {
        gtk::ApplicationWindow {
            set_title: Some("Simple app"),
            set_default_width: 300,
            set_default_height: 100,
            set_child = Some(&gtk::Box) {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,

                append: username = &gtk::Entry {
                    set_placeholder_text: Some("username")
                },

                append: password = &gtk::PasswordEntry{
                    set_placeholder_text: Some("password")
                },

                append = &gtk::Button {
                    set_label: "Login",
                    connect_clicked(sender, username, password) => move |_| {
                        send!(sender, AppMsg::Login{
                            username: username.text().to_string(),
                            password: SecretString::new(password.text().to_string())
                        });
                        username.set_text("");
                        password.set_text("");
                    },
                },


                append = &gtk::Button {
                    set_label: "Increment",
                    connect_clicked(sender) => move |_| {
                        send!(sender, AppMsg::Increment);
                    },
                },
                append = &gtk::Button::with_label("Decrement") {
                    connect_clicked(sender) => move |_| {
                        send!(sender, AppMsg::Decrement);
                    },
                },
                append = &gtk::Label {
                    set_margin_all: 5,
                    set_label: watch! { &format!("Counter: {}", model.counter) },
                }
            },
        }
    }
}

fn main() {
    gtk::init().expect("Failed to initialize GTK!");
    let model = AppModel::new();
    let app = RelmApp::new(model);
    app.run();
}
