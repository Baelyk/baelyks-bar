use iced::futures::{SinkExt, Stream};
use log::warn;

use crate::POLL_RATE_MS;

pub fn battery() -> impl Stream<Item = BatteryMessage> {
    iced::stream::channel(100, async move |mut output| {
        let Ok(manager) = starship_battery::Manager::new() else {
            warn!("Unable to get battery manager");
            return;
        };
        let Ok(mut batteries) = manager.batteries() else {
            warn!("Unable to get batteries");
            return;
        };
        let Some(Ok(mut battery)) = batteries.next() else {
            warn!("Unable to get battery");
            return;
        };

        tokio::task::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(POLL_RATE_MS));
            loop {
                output
                    .send(BatteryMessage::Update((&battery).into()))
                    .await
                    .expect("Unable to send update");
                interval.tick().await;

                if manager.refresh(&mut battery).is_err() {
                    warn!("Unable to refresh battery");
                    break;
                }
            }
        });
    })
}

#[derive(Debug, Copy, Clone)]
pub enum BatteryMessage {
    Update(BatteryInfo),
}

#[derive(Debug, Copy, Clone)]
pub struct BatteryInfo {
    pub charge: u32,
    pub state: starship_battery::State,
    pub char: char,
}

impl From<&starship_battery::Battery> for BatteryInfo {
    fn from(battery: &starship_battery::Battery) -> Self {
        // Get charge as a two-digit percent
        let charge = (battery.state_of_charge().value * 100.0).floor() as u32;
        Self {
            charge,
            state: battery.state(),
            char: battery_char(charge, battery.state()),
        }
    }
}

fn battery_char(charge: u32, state: starship_battery::State) -> char {
    let index = (charge / 10) as usize;
    match state {
        starship_battery::State::Charging => {
            ['󰢟', '󰢜', '󰂆', '󰂇', '󰂈', '󰢝', '󰂉', '󰢞', '󰂊', '󰂋', '󰂅'][index]
        }
        _ => ['󰂃', '󰁻', '󰁼', '󰁽', '󰁽', '󰁾', '󰁿', '󰂀', '󰂁', '󰂂', '󰁹'][index],
    }
}
