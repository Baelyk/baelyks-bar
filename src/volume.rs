use std::process::Command;

use iced::futures::{SinkExt, Stream};

use crate::POLL_RATE_MS;

pub fn volume() -> impl Stream<Item = VolumeMessage> {
    iced::stream::channel(100, async move |mut output| {
        tokio::task::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(POLL_RATE_MS));

            loop {
                output
                    .send(VolumeMessage::Update(
                        get_info().expect("Unable to get volume info"),
                    ))
                    .await
                    .expect("Unable to send update");
                interval.tick().await;
            }
        });
    })
}

#[derive(Debug, Copy, Clone)]
pub enum VolumeMessage {
    Update(VolumeInfo),
}

#[derive(Debug, Copy, Clone)]
pub struct VolumeInfo {
    pub volume: u32,
    pub muted: bool,
}

fn get_info() -> Result<VolumeInfo, Box<dyn std::error::Error>> {
    let stdout = String::try_from(
        Command::new("wpctl")
            .arg("get-volume")
            .arg("@DEFAULT_AUDIO_SINK@")
            .output()?
            .stdout,
    )?;

    let volume = stdout
        .matches(char::is_numeric)
        .collect::<String>()
        .parse::<u32>()?;

    let muted = stdout.contains("MUTED");

    Ok(VolumeInfo { volume, muted })
}
