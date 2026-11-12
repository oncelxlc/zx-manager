mod publisher;

use super::Source;
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::PROPERTYKEY,
    Storage::EnhancedStorage::{PKEY_AppUserModel_ID, PKEY_Link_TargetParsingPath},
    System::Com::CoTaskMemFree,
    UI::Shell::{
        BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, IShellItem2,
        SHGetKnownFolderItem, KF_FLAG_DEFAULT,
    },
};
use windows_core::Interface;

type Observed = BTreeSet<(String, PathBuf)>;

#[derive(Default)]
pub(super) struct Sources {
    verified: HashMap<String, Source>,
    observed: Observed,
    refreshed: Option<Instant>,
}

impl Sources {
    pub(super) fn refresh(&mut self, observed: &Observed) {
        if let Some(previous) = self.refreshed {
            let age = previous.elapsed();
            if age < Duration::from_secs(30)
                || (age < Duration::from_secs(300) && observed == &self.observed)
            {
                return;
            }
        }
        self.refreshed = Some(Instant::now());
        self.observed.clone_from(observed);
        self.verified = resolve(observed, registrations, publisher::verified_client);
    }

    pub(super) fn lookup(&self, id: &str) -> Option<Source> {
        self.verified.get(id).copied()
    }
}

type PropertySink<'a> = dyn FnMut(String, &Path) + 'a;

fn resolve(
    observed: &Observed,
    enumerate: impl FnOnce(&mut PropertySink<'_>) -> windows_core::Result<()>,
    verify: impl Fn(&Path, Source) -> bool,
) -> HashMap<String, Source> {
    let mut owners = HashMap::new();
    let mut checked = HashMap::new();
    let mut record = |id: String, path: &Path| {
        let source = Source::from_filename(path).and_then(|candidate| {
            *checked
                .entry(path.to_path_buf())
                .or_insert_with(|| verify(path, candidate).then_some(candidate))
        });
        claim(&mut owners, id, source);
    };
    for (id, path) in observed {
        record(id.clone(), path);
    }
    if enumerate(&mut record).is_err() {
        // Incomplete enumeration cannot rule out conflicting owners.
        return HashMap::new();
    }
    owners
        .into_iter()
        .filter_map(|(id, owner)| Some((id, owner?)))
        .collect()
}

fn claim(owners: &mut HashMap<String, Option<Source>>, id: String, source: Option<Source>) {
    if id.is_empty() {
        return;
    }
    let previous = owners.entry(id).or_insert(source);
    if *previous != source {
        *previous = None;
    }
}

fn property(item: &IShellItem2, key: &PROPERTYKEY) -> Option<String> {
    unsafe {
        let text = item.GetString(key).ok()?;
        let result = text.to_string().ok();
        CoTaskMemFree(Some(text.0.cast()));
        result
    }
}

fn registrations(add: &mut PropertySink<'_>) -> windows_core::Result<()> {
    unsafe {
        let folder: IShellItem = SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None)?;
        let items: IEnumShellItems = folder.BindToHandler(None, &BHID_EnumItems)?;
        loop {
            let mut batch: [Option<IShellItem>; 16] = Default::default();
            let mut count = 0;
            items.Next(&mut batch, Some(&mut count))?;
            for item in batch.into_iter().take(count as usize).flatten() {
                let item: IShellItem2 = item.cast()?;
                if let Some(id) = property(&item, &PKEY_AppUserModel_ID) {
                    let target = property(&item, &PKEY_Link_TargetParsingPath).unwrap_or_default();
                    add(id, Path::new(&target));
                }
            }
            if count < 16 {
                return Ok(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_registered_clients_without_windows_and_verifies_each_candidate_once() {
        use std::cell::Cell;
        let checks = Cell::new(0);
        let sources = resolve(
            &Observed::new(),
            |add| {
                add("QQ".into(), Path::new(r"C:\Vendor\QQ.exe"));
                add("QQ".into(), Path::new(r"C:\Vendor\QQ.exe"));
                add("different".into(), Path::new(r"C:\Other\QQ.exe"));
                add("helper".into(), Path::new(r"C:\Vendor\QQUpdater.exe"));
                add(
                    "registered-weixin".into(),
                    Path::new(r"C:\Vendor\Weixin.exe"),
                );
                Ok(())
            },
            |path, _| {
                checks.set(checks.get() + 1);
                path.starts_with(r"C:\Vendor")
            },
        );
        assert_eq!(checks.get(), 3);
        assert_eq!(sources.len(), 2);
        assert_eq!(sources["QQ"], Source::Qq);
        assert_eq!(sources["registered-weixin"], Source::Wechat);
    }

    #[test]
    fn conflicting_window_and_shell_claims_are_rejected_in_both_orders() {
        let observed = Observed::from([
            ("mixed".into(), PathBuf::from("QQ.exe")),
            ("reused".into(), PathBuf::from("QQ.exe")),
        ]);
        let sources = resolve(
            &observed,
            |add| {
                add("mixed".into(), Path::new("Weixin.exe"));
                add("reused".into(), Path::new("other.exe"));
                add("reused".into(), Path::new("QQ.exe"));
                add("reverse".into(), Path::new(""));
                add("reverse".into(), Path::new("QQ.exe"));
                Ok(())
            },
            |_, _| true,
        );
        assert!(sources.is_empty());
    }

    #[test]
    #[ignore = "requires signed clients installed in the interactive Windows session"]
    fn resolves_installed_clients_without_windows() {
        use ::windows::Win32::System::WinRT::{
            RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED,
        };
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.unwrap();
        let mut sources = Sources::default();
        sources.refresh(&Observed::new());
        unsafe { RoUninitialize() };
        for (variable, expected) in [
            ("ZX_ONCE_WECHAT_AUMID", Source::Wechat),
            ("ZX_ONCE_QQ_AUMID", Source::Qq),
        ] {
            let id = std::env::var(variable)
                .expect("set the AUMID from this client's Shell registration");
            assert_eq!(sources.lookup(&id), Some(expected));
        }
    }
}
