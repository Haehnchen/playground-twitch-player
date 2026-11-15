use super::TwitchStreamQuality;
use gtk4::{gio, glib};
use std::ffi::CString;

fn quality_label(height: u32, frame_rate: f64, bandwidth: u32) -> String {
    if height > 0 {
        if frame_rate >= 50.0 {
            format!("{height}p60")
        } else {
            format!("{height}p")
        }
    } else if bandwidth > 0 {
        format!("{} kbps", std::cmp::max(1, bandwidth / 1000))
    } else {
        "Unknown".to_owned()
    }
}

// Read video variants and sort them for the quality picker.
pub(super) fn parse_stream_qualities_playlist(
    bytes: &[u8],
) -> Result<Vec<TwitchStreamQuality>, glib::Error> {
    let playlist = m3u8_rs::parse_master_playlist_res(bytes)
        .map_err(|_| glib::Error::new(gio::IOErrorEnum::NotFound, "Invalid HLS master playlist"))?;
    let mut qualities = Vec::new();
    for variant in playlist.variants {
        let Some(resolution) = variant.resolution else {
            continue;
        };
        let audio_only = variant
            .other_attributes
            .as_ref()
            .and_then(|attrs| attrs.get("NAME"))
            .is_some_and(|name| name.to_string().to_ascii_lowercase().contains("audio"));
        let (Ok(width), Ok(height)) = (
            u32::try_from(resolution.width),
            u32::try_from(resolution.height),
        ) else {
            continue;
        };
        if height == 0 || variant.is_i_frame || audio_only {
            continue;
        }
        let bandwidth = variant.bandwidth.try_into().unwrap_or(0);
        let frame_rate = variant.frame_rate.unwrap_or(0.0);
        qualities.push(TwitchStreamQuality {
            label: CString::new(quality_label(height, frame_rate, bandwidth)).unwrap(),
            url: CString::new(variant.uri.split('\0').next().unwrap_or_default()).unwrap(),
            width,
            height,
            bandwidth,
            frame_rate,
        });
    }
    qualities.sort_by(|a, b| {
        a.height
            .cmp(&b.height)
            .then_with(|| a.frame_rate.total_cmp(&b.frame_rate))
            .then_with(|| a.bandwidth.cmp(&b.bandwidth))
            .then_with(|| {
                a.label
                    .as_bytes()
                    .to_ascii_lowercase()
                    .cmp(&b.label.as_bytes().to_ascii_lowercase())
            })
    });
    if qualities.is_empty() {
        Err(glib::Error::new(
            gio::IOErrorEnum::NotFound,
            "No HLS stream variants found",
        ))
    } else {
        Ok(qualities)
    }
}
