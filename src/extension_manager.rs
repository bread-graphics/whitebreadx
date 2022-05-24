// MIT/Apache2 License

use crate::sync::{rwl_read, rwl_write, RwLock};
use breadx::{
    display::{Display, DisplayFunctionsExt},
    protocol::{ExtInfoProvider, ExtensionInformation},
    Result,
};
use core::mem;

pub(crate) struct ExtensionManager {
    entries: RwLock<HashMap<&'static str, Option<ExtensionInformation>>>,
}

impl ExtensionManager {
    pub(crate) fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::with_hasher(Default::default())),
        }
    }

    pub(crate) fn extension_code(
        &self,
        display: &mut impl Display,
        name: &'static str,
    ) -> Result<Option<u8>> {
        // fast path: do we already have it
        let guard = rwl_read(&self.entries);

        if let Some(entry) = guard.get(&name) {
            return Ok(entry.as_ref().map(|entry| entry.major_opcode));
        }

        // slow path: we don't have it, so we need to query it
        mem::drop(guard);
        let mut guard = rwl_write(&self.entries);

        // someone else may have queried it while we were waiting
        // check if so
        if let Some(entry) = guard.get(&name) {
            return Ok(entry.as_ref().map(|entry| entry.major_opcode));
        }

        let res = display.query_extension_immediate(name)?;
        let ext_info = if res.present {
            Some(ExtensionInformation {
                major_opcode: res.major_opcode,
                first_event: res.first_event,
                first_error: res.first_error,
            })
        } else {
            None
        };

        guard.insert(name, ext_info);
        Ok(Some(res.major_opcode).filter(|_| res.present))
    }

    fn find_extension_info(
        &self,
        mut f: impl FnMut(&ExtensionInformation) -> bool,
    ) -> Option<(&'static str, ExtensionInformation)> {
        let guard = rwl_read(&self.entries);

        for (name, qer) in guard.iter() {
            if let Some(qer) = qer {
                if f(qer) {
                    return Some((name, *qer));
                }
            }
        }

        None
    }
}

impl ExtInfoProvider for ExtensionManager {
    fn get_from_error_code(&self, error_code: u8) -> Option<(&str, ExtensionInformation)> {
        self.find_extension_info(|qer| qer.first_error == error_code)
    }

    fn get_from_event_code(&self, event_code: u8) -> Option<(&str, ExtensionInformation)> {
        self.find_extension_info(|qer| qer.first_event == event_code)
    }

    fn get_from_major_opcode(&self, major_opcode: u8) -> Option<(&str, ExtensionInformation)> {
        self.find_extension_info(|qer| qer.major_opcode == major_opcode)
    }
}

type HashMap<K, V> = hashbrown::HashMap<K, V, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>;
