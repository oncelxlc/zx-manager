use super::notifications::AppIcon;
use serde::Serialize;
use std::{
    sync::{mpsc, Arc},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use windows::{
    core::Result,
    Graphics::Imaging::*,
    Media::{
        Control::{
            GlobalSystemMediaTransportControlsSession as Session,
            GlobalSystemMediaTransportControlsSessionManager as SessionManager,
            GlobalSystemMediaTransportControlsSessionMediaProperties as Properties,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
        },
        MediaPlaybackType,
    },
    Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
};
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaSnapshot {
    #[serde(skip)]
    pub source: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub can_previous: bool,
    pub can_toggle: bool,
    pub can_next: bool,
    pub can_seek: bool,
    pub cover: Option<Arc<AppIcon>>,
    pub battery: Option<(u8, bool)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Transport {
    Previous,
    Toggle,
    Next,
    Seek(u16), // thousandths of the displayed track
}

pub(crate) struct Worker(mpsc::Sender<(String, String, Transport)>);

impl Worker {
    pub fn start(emit: impl Fn(MediaSnapshot) + Send + 'static) -> std::io::Result<Self> {
        let (send, receive) = mpsc::channel::<(String, String, Transport)>();
        thread::Builder::new()
            .name("island-media".into())
            .spawn(move || {
                let initialized = unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.is_ok();
                let mut manager = None;
                let mut last = MediaSnapshot::default();
                loop {
                    if manager.is_none() && initialized {
                        manager = SessionManager::RequestAsync().and_then(|op| op.get()).ok();
                    }
                    let session = manager.as_ref().and_then(select_session);
                    let mut snapshot = session
                        .as_ref()
                        .and_then(|s| read(s, &last).ok())
                        .unwrap_or_default();
                    snapshot.battery = battery();
                    if snapshot != last {
                        last = snapshot;
                        emit(last.clone());
                    }
                    let delay = if last.playing { 1 } else { 3 };
                    match receive.recv_timeout(Duration::from_secs(delay)) {
                        Ok((source, title, action)) => {
                            if let Some(s) =
                                session.filter(|_| source == last.source && title == last.title)
                            {
                                if let Err(e) = control(&s, &title, action) {
                                    eprintln!("音乐播放器未执行操作：{e}");
                                }
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                }
                drop(manager);
                if initialized {
                    unsafe { RoUninitialize() }
                }
            })?;
        Ok(Self(send))
    }

    pub fn control(
        &self,
        track: &MediaSnapshot,
        action: Transport,
    ) -> std::result::Result<(), String> {
        self.0
            .send((track.source.clone(), track.title.clone(), action))
            .map_err(|_| "媒体监听已停止".into())
    }
}

fn select_session(manager: &SessionManager) -> Option<Session> {
    let audio = |s: &Session| {
        s.GetPlaybackInfo()
            .and_then(|p| p.PlaybackType())
            .and_then(|t| t.Value())
            .is_ok_and(|kind| kind == MediaPlaybackType::Music)
    };
    let current = manager.GetCurrentSession().ok();
    if current.as_ref().is_some_and(audio) {
        return current;
    }
    let sessions = manager.GetSessions().ok();
    sessions
        .and_then(|list| list.into_iter().find(audio))
        .or(current)
}

fn battery() -> Option<(u8, bool)> {
    let mut status = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return None;
    }
    battery_reading(&status)
}

fn battery_reading(status: &SYSTEM_POWER_STATUS) -> Option<(u8, bool)> {
    // A plugged-in laptop can stop charging at its configured charge limit.
    (status.BatteryFlag & 128 == 0 && status.BatteryLifePercent <= 100)
        .then_some((status.BatteryLifePercent, status.ACLineStatus == 1))
}

fn read(session: &Session, last: &MediaSnapshot) -> Result<MediaSnapshot> {
    let info = session.GetPlaybackInfo()?;
    let properties = session.TryGetMediaPropertiesAsync()?.get()?;
    let kind = properties
        .PlaybackType()
        .or_else(|_| info.PlaybackType())
        .and_then(|v| v.Value())
        .ok();
    let title = properties.Title()?.to_string();
    if title.trim().is_empty()
        || matches!(
            kind,
            Some(MediaPlaybackType::Video | MediaPlaybackType::Image)
        )
    {
        return Ok(MediaSnapshot::default());
    }
    let source = session.SourceAppUserModelId()?.to_string();
    let artist = properties.Artist().unwrap_or_default().to_string();
    let playing = info.PlaybackStatus()? == Status::Playing;
    let controls = info.Controls()?;
    let mut result = MediaSnapshot {
        cover: if last.source == source
            && last.title == title
            && last.artist == artist
            && last.cover.is_some()
        {
            last.cover.clone()
        } else {
            artwork(&properties).ok().map(Arc::new)
        },
        source,
        title,
        artist,
        playing,
        can_previous: controls.IsPreviousEnabled().unwrap_or(false),
        can_next: controls.IsNextEnabled().unwrap_or(false),
        can_toggle: controls.IsPlayPauseToggleEnabled().unwrap_or(false)
            || if playing {
                controls.IsPauseEnabled()
            } else {
                controls.IsPlayEnabled()
            }
            .unwrap_or(false),
        ..Default::default()
    };
    if let Ok(timeline) = session.GetTimelineProperties() {
        let start = timeline.StartTime().unwrap_or_default().Duration;
        let end = timeline.EndTime().unwrap_or_default().Duration;
        result.duration_ms = end.saturating_sub(start).max(0) / 10_000;
        let stamp = timeline.LastUpdatedTime().unwrap_or_default().UniversalTime;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
            + 11_644_473_600_000;
        let elapsed = if playing && stamp > 0 {
            (now - stamp / 10_000).max(0)
        } else {
            0
        };
        let rate = info.PlaybackRate().and_then(|r| r.Value()).unwrap_or(1.0);
        let position = timeline
            .Position()
            .unwrap_or_default()
            .Duration
            .saturating_sub(start)
            / 10_000;
        result.position_ms = position
            .saturating_add((elapsed as f64 * rate) as i64)
            .max(0);
        if result.duration_ms > 0 {
            result.position_ms = result.position_ms.min(result.duration_ms);
        }
        result.can_seek =
            result.duration_ms > 0 && controls.IsPlaybackPositionEnabled().unwrap_or(false);
    }
    Ok(result)
}

fn artwork(props: &Properties) -> Result<AppIcon> {
    let stream = props.Thumbnail()?.OpenReadAsync()?.get()?;
    let decoder = BitmapDecoder::CreateAsync(&stream)?.get()?;
    let (w, h) = (decoder.PixelWidth()?, decoder.PixelHeight()?);
    let side = w.max(h).max(1);
    let scale = |v: u32| (u64::from(v) * u64::from(side.min(128)) / u64::from(side)).max(1) as u32;
    let (width, height) = (scale(w), scale(h));
    let transform = BitmapTransform::new()?;
    transform.SetScaledWidth(width)?;
    transform.SetScaledHeight(height)?;
    let pixels = decoder
        .GetPixelDataTransformedAsync(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Premultiplied,
            &transform,
            ExifOrientationMode::IgnoreExifOrientation,
            ColorManagementMode::DoNotColorManage,
        )?
        .get()?
        .DetachPixelData()?;
    Ok(AppIcon {
        width,
        height,
        bgra: pixels.to_vec(),
    })
}

fn control(session: &Session, title: &str, action: Transport) -> Result<()> {
    // A slow player can change tracks between the pointer press and the command.
    if session.TryGetMediaPropertiesAsync()?.get()?.Title()? != title {
        return Ok(());
    }
    let info = session.GetPlaybackInfo()?;
    let op = match action {
        Transport::Previous => session.TrySkipPreviousAsync()?,
        Transport::Next => session.TrySkipNextAsync()?,
        Transport::Toggle if info.Controls()?.IsPlayPauseToggleEnabled()? => {
            session.TryTogglePlayPauseAsync()?
        }
        Transport::Toggle if info.PlaybackStatus()? == Status::Playing => {
            session.TryPauseAsync()?
        }
        Transport::Toggle => session.TryPlayAsync()?,
        Transport::Seek(fraction) => {
            let timeline = session.GetTimelineProperties()?;
            let start = timeline.StartTime()?.Duration;
            let end = timeline.EndTime()?.Duration;
            let to = start.saturating_add(
                (end.saturating_sub(start) as f64 * f64::from(fraction.min(1000)) / 1000.0) as i64,
            );
            session.TryChangePlaybackPositionAsync(to)?
        }
    };
    if !op.get()? {
        return Err(windows::core::Error::new(
            windows::Win32::Foundation::E_FAIL,
            "播放器拒绝此操作",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod battery_tests {
    use super::*;

    #[test]
    fn plugged_in_at_charge_limit_uses_the_power_indicator() {
        let mut status = SYSTEM_POWER_STATUS {
            ACLineStatus: 1,
            BatteryFlag: 1,
            BatteryLifePercent: 78,
            ..Default::default()
        };
        assert_eq!(battery_reading(&status), Some((78, true)));
        status.ACLineStatus = 0;
        assert_eq!(battery_reading(&status), Some((78, false)));
        status.BatteryFlag = 128;
        assert!(battery_reading(&status).is_none());
        status.BatteryFlag = 1;
        status.BatteryLifePercent = 255;
        assert!(battery_reading(&status).is_none());
    }
}
