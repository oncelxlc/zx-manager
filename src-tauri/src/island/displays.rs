use super::{
    config::IslandConfig,
    geometry::{Display, Rect},
    state::IslandState,
};
use tauri::{AppHandle, Monitor};

type Result<T> = std::result::Result<T, String>;

fn display(monitor: &Monitor, primary: Option<&Monitor>) -> Display {
    let pos = monitor.position();
    let name = monitor.name().cloned().unwrap_or_else(|| {
        format!(
            "{} × {} @ {},{}",
            monitor.size().width,
            monitor.size().height,
            pos.x,
            pos.y
        )
    });
    let work = monitor.work_area();
    Display {
        id: name.clone(),
        name,
        primary: primary.is_some_and(|p| p.position() == pos),
        scale_factor: monitor.scale_factor(),
        work: Rect {
            x: work.position.x,
            y: work.position.y,
            width: work.size.width.max(1) as i32,
            height: work.size.height.max(1) as i32,
        },
    }
}

pub(super) fn displays(app: &AppHandle) -> Result<Vec<Display>> {
    let primary = app.primary_monitor().map_err(|e| e.to_string())?;
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    Ok(monitors
        .iter()
        .map(|m| display(m, primary.as_ref()))
        .collect())
}

pub(super) fn drag_position(
    app: &AppHandle,
    cfg: &IslandConfig,
    state: IslandState,
    at: Rect,
) -> Result<(String, f64, f64)> {
    let screen = displays(app)?
        .into_iter()
        .max_by_key(|d| overlap_and_distance(d.work, at))
        .ok_or("找不到可用显示器")?;
    let (x, y) = screen.drag_offset(cfg, state, at);
    Ok((screen.id, x, y))
}

// A partially off-screen island can have its center in a gap between monitors.
fn overlap_and_distance(work: Rect, island: Rect) -> (i64, std::cmp::Reverse<i128>) {
    let overlap = |a, length_a, b, length_b| {
        let (a, b) = (i64::from(a), i64::from(b));
        (a + i64::from(length_a)).min(b + i64::from(length_b)) - a.max(b)
    };
    let dx = overlap(work.x, work.width, island.x, island.width);
    let dy = overlap(work.y, work.height, island.y, island.height);
    let gap_squared = i128::from(dx.min(0)).pow(2) + i128::from(dy.min(0)).pow(2);
    (dx.max(0) * dy.max(0), std::cmp::Reverse(gap_squared))
}

pub(super) fn target_display<'a>(screens: &'a [Display], id: Option<&str>) -> Result<&'a Display> {
    screens
        .iter()
        .find(|s| Some(s.id.as_str()) == id)
        .or_else(|| screens.iter().find(|s| s.primary))
        .or_else(|| screens.first())
        .ok_or_else(|| "找不到可用显示器".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_prefers_the_largest_overlap_then_the_nearest_work_area() {
        let left = Rect {
            x: -1000,
            y: 0,
            width: 800,
            height: 600,
        };
        let right = Rect { x: 0, ..left };
        // Center (-75, 120) is in the gap, with 25 px on the left and 75 px on the right.
        let island = Rect {
            x: -225,
            y: 100,
            width: 300,
            height: 40,
        };
        assert!(overlap_and_distance(right, island) > overlap_and_distance(left, island));
        let in_gap = Rect {
            x: -180,
            width: 30,
            ..island
        };
        assert!(overlap_and_distance(left, in_gap) > overlap_and_distance(right, in_gap));
    }

    #[test]
    fn unplugged_display_falls_back_without_changing_the_saved_choice() {
        let display = |id: &str, primary| Display {
            id: id.into(),
            name: id.into(),
            primary,
            scale_factor: 1.0,
            work: Rect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
        };
        let screens = [display("left", false), display("right", true)];
        assert_eq!(target_display(&screens, Some("left")).unwrap().id, "left");
        assert_eq!(target_display(&screens, Some("gone")).unwrap().id, "right");
        assert_eq!(
            target_display(&screens[..1], Some("gone")).unwrap().id,
            "left"
        );
        assert!(target_display(&[], None).is_err());
    }
}
