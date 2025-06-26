use chrono::Local;
use iced::{
    Element, Length, Subscription, Task, Theme,
    widget::{self, Row, button, center_y, mouse_area, row, text},
};
use iced_layershell::{
    Settings, application, reexport::Anchor, settings::LayerShellSettings, to_layer_message,
};
use log::{trace, warn};

use crate::{
    battery::{self, BatteryInfo, BatteryMessage},
    sway::SwayMessenger,
    volume::{VolumeInfo, VolumeMessage},
};
use crate::{
    sway::{self, SwayMessage, WorkspaceInfo},
    volume,
};

const HEIGHT: u32 = 40;
const TEXT_SIZE: f32 = 20.0;
const SMALL: f32 = 12.0;
const MEDIUM: f32 = 24.0;

pub fn run() -> Result<(), iced_layershell::Error> {
    application(State::default, State::namespace, State::update, State::view)
        .subscription(State::subscription)
        .style(State::style)
        .theme(State::theme)
        .settings(Settings {
            layer_settings: LayerShellSettings {
                anchor: Anchor::Top,
                size: Some((2880, HEIGHT)),
                exclusive_zone: HEIGHT as i32,
                ..Default::default()
            },
            default_font: iced::Font::with_name("JetBrainsMono Nerd Font"),
            ..Default::default()
        })
        .run()
}

#[derive(Default)]
struct State {
    clock_hovered: bool,
    workspaces: Vec<WorkspaceInfo>,
    sway_messenger: Option<SwayMessenger>,
    battery: Option<BatteryInfo>,
    battery_hovered: bool,
    volume: Option<VolumeInfo>,
}

#[to_layer_message(multi)]
#[derive(Clone, Debug)]
enum Message {
    Tick,
    ClockHover(bool),
    Sway(SwayMessage),
    SwitchWorkspace(i32),
    Battery(BatteryMessage),
    BatteryHover(bool),
    Volume(VolumeMessage),
}

impl State {
    fn namespace() -> String {
        String::from("Bar")
    }

    fn workspaces(&self) -> Element<Message> {
        center_y(
            Row::from_iter(self.workspaces.iter().map(|info| {
                button("")
                    .on_press(Message::SwitchWorkspace(info.num))
                    .style(|theme: &Theme, _| iced::widget::button::Style {
                        background: if info.urgent {
                            Some(theme.palette().danger.into())
                        } else if info.focused {
                            Some(theme.palette().primary.into())
                        } else if info.nonempty {
                            Some(theme.palette().text.into())
                        } else {
                            None
                        },
                        border: iced::Border::default()
                            .width(2)
                            .rounded(3)
                            .color(if info.urgent {
                                theme.palette().danger
                            } else if info.focused || info.visible {
                                theme.palette().primary
                            } else {
                                theme.palette().text
                            }),
                        ..Default::default()
                    })
                    .width(SMALL)
                    .height(SMALL)
                    .into()
            }))
            .spacing(MEDIUM)
            .padding([0.0, MEDIUM]),
        )
        .into()
    }

    fn clock(&self) -> Element<Message> {
        let format = if self.clock_hovered { "%c" } else { "%H:%M" };
        let time = Local::now().format(format);
        mouse_area(center_y(text(time.to_string()).size(TEXT_SIZE)).padding([0.0, SMALL]))
            .on_enter(Message::ClockHover(true))
            .on_exit(Message::ClockHover(false))
            .into()
    }

    fn battery(&self) -> Option<Element<Message>> {
        let info = self.battery?;

        let content = if self.battery_hovered {
            format!("{} {}%", info.char, info.charge)
        } else {
            format!("{}", info.char)
        };

        let color = move |theme: &Theme| widget::text::Style {
            color: if info.state == starship_battery::State::Charging
                || info.state == starship_battery::State::Full
            {
                Some(theme.palette().success)
            } else if info.charge <= 10 {
                Some(theme.palette().danger)
            } else {
                None
            },
        };

        Some(
            mouse_area(center_y(text(content).style(color).size(TEXT_SIZE)).padding([0.0, SMALL]))
                .on_enter(Message::BatteryHover(true))
                .on_exit(Message::BatteryHover(false))
                .into(),
        )
    }

    fn volume(&self) -> Option<Element<Message>> {
        let info = self.volume?;

        let icon = if info.muted { '' } else { '' };

        Some(
            center_y(text(format!("{} {:>3}%", icon, info.volume)).size(TEXT_SIZE))
                .padding([0.0, SMALL])
                .into(),
        )
    }

    fn view(&self) -> Element<Message> {
        let left = row![self.workspaces()];

        let right = Row::new()
            .spacing(SMALL)
            .push_maybe(self.volume())
            .push_maybe(self.battery())
            .push(self.clock());
        let right = widget::right(right);

        row![left, right].width(Length::Fill).into()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        trace!("Update {:#?}", message);
        match message {
            Message::Tick => Task::none(),
            Message::ClockHover(hovered) => {
                self.clock_hovered = hovered;
                Task::none()
            }
            Message::Sway(message) => {
                match message {
                    SwayMessage::Initialized(sway_messenger) => {
                        self.sway_messenger = Some(sway_messenger)
                    }
                    SwayMessage::Workspaces(workspaces) => {
                        self.workspaces = workspaces;
                    }
                }
                Task::none()
            }
            Message::SwitchWorkspace(num) => match &mut self.sway_messenger {
                Some(sway_messenger) => {
                    sway_messenger.switch_workspace(num);
                    Task::none()
                }
                None => {
                    warn!("Unable to send SwitchWorkspace({num}), SwayMessenger uninitialized");
                    Task::none()
                }
            },
            Message::Battery(message) => {
                match message {
                    BatteryMessage::Update(info) => {
                        self.battery = Some(info);
                    }
                }
                Task::none()
            }
            Message::BatteryHover(hovered) => {
                self.battery_hovered = hovered;
                Task::none()
            }
            Message::Volume(message) => {
                match message {
                    VolumeMessage::Update(info) => {
                        self.volume = Some(info);
                    }
                }
                Task::none()
            }
            _ => {
                warn!("Unexpected message {:?}", message);
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let sway = Subscription::run(sway::sway).map(Message::Sway);
        let battery = Subscription::run(battery::battery).map(Message::Battery);
        let volume = Subscription::run(volume::volume).map(Message::Volume);
        Subscription::batch([sway, battery, volume])
    }

    fn style(&self, theme: &Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: theme.palette().background,
            text_color: theme.palette().text,
        }
    }

    fn theme(&self) -> Theme {
        iced::Theme::custom(
            "Gruvbox Dark".to_string(),
            iced::theme::Palette {
                text: iced::color!(0xebdbb2),
                ..iced::theme::Palette::GRUVBOX_DARK
            },
        )
    }
}
