use super::SurfaceEvent;
use crate::island::{
    config::IslandConfig,
    geometry::Rect,
    render::{MessageAction, Panel},
    state::IslandState,
};
use std::sync::{Arc, RwLock};
use windows::{
    core::{implement, IUnknownImpl, Interface, Result, BSTR, GUID, PCWSTR},
    Win32::{
        Foundation::{
            CO_E_OBJNOTCONNECTED, E_INVALIDARG, E_NOTIMPL, E_POINTER, HWND, S_FALSE, WPARAM,
        },
        System::{
            Com::{IDispatch, IDispatch_Impl, ITypeInfo, DISPATCH_FLAGS, DISPPARAMS, EXCEPINFO},
            Ole::DispInvoke,
            Variant::{VARIANT, VT_I4},
        },
        UI::{Accessibility::*, WindowsAndMessaging::OBJID_CLIENT},
    },
};
use windows_sys::Win32::UI::{
    Controls::STATE_SYSTEM_UNAVAILABLE,
    WindowsAndMessaging::{STATE_SYSTEM_COLLAPSED, STATE_SYSTEM_EXPANDED},
};

#[derive(Clone, PartialEq)]
struct Control {
    name: String,
    rect: Rect,
    action: Option<SurfaceEvent>,
    state: u32,
    page: bool,
}

type Snapshot = Arc<RwLock<Vec<Control>>>;
type Notify = Arc<dyn Fn(SurfaceEvent) + Send + Sync>;

pub(super) struct Accessibility {
    controls: Snapshot,
    notify: Notify,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_body_opens_and_expanded_body_exposes_no_management_controls() {
        let a11y = Accessibility::new(Arc::new(|_| {}));
        let cfg = IslandConfig::default();
        let bounds = Rect {
            x: -328,
            y: 0,
            width: 328,
            height: 168,
        };
        a11y.update_panel(&cfg, IslandState::Compact, bounds, None);
        assert_eq!(
            a11y.controls.read().unwrap()[1].action,
            Some(SurfaceEvent::Click)
        );
        a11y.update_panel(&cfg, IslandState::Expanded, bounds, None);
        assert_eq!(a11y.controls.read().unwrap().len(), 1);
        a11y.clear();
        assert!(a11y.controls.read().unwrap().is_empty());
    }
}

impl Accessibility {
    pub(super) fn new(notify: Notify) -> Self {
        Self {
            controls: Default::default(),
            notify,
        }
    }

    pub(super) fn update_panel(
        &self,
        cfg: &IslandConfig,
        state: IslandState,
        rect: Rect,
        panel: Option<&Panel>,
    ) -> bool {
        let mut nodes = Vec::with_capacity(4);
        let expanded = matches!(state, IslandState::Expanded | IslandState::Pinned);
        let body_action = state == IslandState::EdgePeek
            || (state == IslandState::Compact && cfg.interaction.click_expand);
        nodes.push(Control {
            name: panel.map_or_else(|| "灵动岛".into(), |p| format!("灵动岛，{}", p.status)),
            rect,
            action: None,
            state: if expanded {
                STATE_SYSTEM_EXPANDED
            } else {
                STATE_SYSTEM_COLLAPSED
            },
            page: false,
        });
        if body_action {
            nodes.push(Control {
                name: if state == IslandState::EdgePeek {
                    "唤醒灵动岛"
                } else {
                    "展开灵动岛"
                }
                .into(),
                rect,
                action: Some(SurfaceEvent::Click),
                page: false,
                state: STATE_SYSTEM_COLLAPSED,
            });
        }
        if let Some(panel) = panel {
            for item in &panel.items {
                nodes.push(Control {
                    name: item.name.clone(),
                    rect: item.screen_bounds(rect, panel.size),
                    action: item.action.and_then(MessageAction::surface_event),
                    page: item.action == Some(MessageAction::NextPage),
                    state: if item.action.is_none() {
                        STATE_SYSTEM_UNAVAILABLE
                    } else {
                        0
                    },
                });
            }
        }
        let mut previous = self.controls.write().unwrap_or_else(|e| e.into_inner());
        if *previous == nodes {
            return false;
        }
        *previous = nodes;
        true
    }

    pub(super) fn clear(&self) {
        self.controls
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    pub(super) fn object(
        &self,
        hwnd: windows_sys::Win32::Foundation::HWND,
        wp: usize,
        lp: isize,
    ) -> Option<isize> {
        if lp as i32 != OBJID_CLIENT.0 {
            return None;
        }
        unsafe {
            let mut standard = std::ptr::null_mut();
            CreateStdAccessibleObject(HWND(hwnd), OBJID_CLIENT.0, &IAccessible::IID, &mut standard)
                .ok()?;
            let provider: IAccessible = IslandAccessible {
                standard: IAccessible::from_raw(standard),
                controls: self.controls.clone(),
                notify: self.notify.clone(),
                hwnd: hwnd as usize,
            }
            .into();
            Some(LresultFromObject(&IAccessible::IID, WPARAM(wp), &provider).0)
        }
    }
}

#[implement(IAccessible)]
struct IslandAccessible {
    standard: IAccessible,
    controls: Snapshot,
    notify: Notify,
    hwnd: usize,
}

impl IslandAccessible {
    fn index(&self, child: &VARIANT) -> Result<usize> {
        if unsafe { child.Anonymous.Anonymous.vt } != VT_I4 {
            return Err(E_INVALIDARG.into());
        }
        i32::try_from(child)?
            .try_into()
            .map_err(|_| E_INVALIDARG.into())
    }

    fn control(&self, child: &VARIANT) -> Result<Control> {
        let nodes = self.controls.read().map_err(|_| CO_E_OBJNOTCONNECTED)?;
        nodes
            .get(self.index(child)?)
            .cloned()
            .ok_or_else(|| E_INVALIDARG.into())
    }
}

#[allow(non_snake_case)]
impl IDispatch_Impl for IslandAccessible_Impl {
    fn GetTypeInfoCount(&self) -> Result<u32> {
        unsafe { self.standard.GetTypeInfoCount() }
    }
    fn GetTypeInfo(&self, index: u32, locale: u32) -> Result<ITypeInfo> {
        unsafe { self.standard.GetTypeInfo(index, locale) }
    }
    fn GetIDsOfNames(
        &self,
        riid: *const GUID,
        names: *const PCWSTR,
        count: u32,
        locale: u32,
        ids: *mut i32,
    ) -> Result<()> {
        unsafe { self.standard.GetIDsOfNames(riid, names, count, locale, ids) }
    }
    fn Invoke(
        &self,
        member: i32,
        _: *const GUID,
        locale: u32,
        flags: DISPATCH_FLAGS,
        params: *const DISPPARAMS,
        result: *mut VARIANT,
        exception: *mut EXCEPINFO,
        error: *mut u32,
    ) -> Result<()> {
        unsafe {
            DispInvoke(
                self.to_interface::<IAccessible>().as_raw(),
                &self.standard.GetTypeInfo(0, locale)?,
                member,
                flags.0,
                params.cast_mut(),
                result,
                exception,
                error,
            )
        }
    }
}

#[allow(non_snake_case)]
impl IAccessible_Impl for IslandAccessible_Impl {
    fn accParent(&self) -> Result<IDispatch> {
        unsafe { self.standard.accParent() }
    }
    fn accChildCount(&self) -> Result<i32> {
        Ok(self
            .controls
            .read()
            .map_err(|_| CO_E_OBJNOTCONNECTED)?
            .len()
            .saturating_sub(1) as i32)
    }
    fn get_accChild(&self, child: &VARIANT) -> Result<IDispatch> {
        self.control(child)?;
        Err(S_FALSE.into())
    }
    fn get_accName(&self, child: &VARIANT) -> Result<BSTR> {
        Ok(BSTR::from(self.control(child)?.name.as_str()))
    }
    fn get_accRole(&self, child: &VARIANT) -> Result<VARIANT> {
        self.control(child)?;
        Ok((if self.index(child)? == 0 {
            ROLE_SYSTEM_PANE
        } else {
            ROLE_SYSTEM_PUSHBUTTON
        } as i32)
            .into())
    }
    fn get_accState(&self, child: &VARIANT) -> Result<VARIANT> {
        Ok((self.control(child)?.state as i32).into())
    }
    fn get_accDefaultAction(&self, child: &VARIANT) -> Result<BSTR> {
        let c = self.control(child)?;
        Ok(if c.page || c.action.is_some() {
            "按下".into()
        } else {
            BSTR::new()
        })
    }
    fn accDoDefaultAction(&self, child: &VARIANT) -> Result<()> {
        let c = self.control(child)?;
        if c.page {
            if unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                    self.hwnd as _,
                    super::NEXT_PAGE,
                    0,
                    0,
                )
            } == 0
            {
                return Err(CO_E_OBJNOTCONNECTED.into());
            }
        } else {
            (self.notify)(c.action.ok_or(E_INVALIDARG)?);
        }
        Ok(())
    }
    fn accLocation(
        &self,
        x: *mut i32,
        y: *mut i32,
        w: *mut i32,
        h: *mut i32,
        child: &VARIANT,
    ) -> Result<()> {
        if [x, y, w, h].iter().any(|p| p.is_null()) {
            return Err(E_POINTER.into());
        }
        let r = self.control(child)?.rect;
        unsafe {
            *x = r.x;
            *y = r.y;
            *w = r.width;
            *h = r.height;
        }
        Ok(())
    }
    fn accHitTest(&self, x: i32, y: i32) -> Result<VARIANT> {
        let controls = self.controls.read().map_err(|_| CO_E_OBJNOTCONNECTED)?;
        Ok(controls
            .iter()
            .rposition(|c| {
                let r = c.rect;
                x >= r.x && y >= r.y && x < r.x + r.width && y < r.y + r.height
            })
            .map(|i| VARIANT::from(i as i32))
            .unwrap_or_default())
    }
    fn accNavigate(&self, dir: i32, child: &VARIANT) -> Result<VARIANT> {
        let id = self.index(child)? as i32;
        let last = self.accChildCount()?;
        let next = match dir as u32 {
            NAVDIR_FIRSTCHILD if id == 0 && last > 0 => 1,
            NAVDIR_LASTCHILD if id == 0 && last > 0 => last,
            NAVDIR_NEXT if id > 0 && id < last => id + 1,
            NAVDIR_PREVIOUS if id > 1 && id <= last => id - 1,
            _ => return Ok(VARIANT::default()),
        };
        Ok(next.into())
    }
    fn accFocus(&self) -> Result<VARIANT> {
        Ok(VARIANT::default())
    }
    fn accSelection(&self) -> Result<VARIANT> {
        Ok(VARIANT::default())
    }
    fn get_accValue(&self, _: &VARIANT) -> Result<BSTR> {
        Ok(BSTR::new())
    }
    fn get_accDescription(&self, _: &VARIANT) -> Result<BSTR> {
        Ok(BSTR::new())
    }
    fn get_accHelp(&self, _: &VARIANT) -> Result<BSTR> {
        Ok(BSTR::new())
    }
    fn get_accKeyboardShortcut(&self, _: &VARIANT) -> Result<BSTR> {
        Ok(BSTR::new())
    }
    fn get_accHelpTopic(&self, _: *mut BSTR, _: &VARIANT) -> Result<i32> {
        Err(E_NOTIMPL.into())
    }
    fn put_accName(&self, _: &VARIANT, _: &BSTR) -> Result<()> {
        Err(E_NOTIMPL.into())
    }
    fn put_accValue(&self, _: &VARIANT, _: &BSTR) -> Result<()> {
        Err(E_NOTIMPL.into())
    }
    fn accSelect(&self, _: i32, _: &VARIANT) -> Result<()> {
        Err(E_NOTIMPL.into())
    }
}
