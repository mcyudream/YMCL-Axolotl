use parking_lot::Mutex;
use std::collections::BTreeSet;

/// Process-owned presentation unlocks survive WebView recreation, never disk.
#[derive(Default)]
pub struct SimpleModeUnlocks(Mutex<BTreeSet<String>>);

impl SimpleModeUnlocks {
    pub fn unlocked_domains(&self) -> Vec<String> {
        self.0.lock().iter().cloned().collect()
    }

    pub fn unlock(&self, domain_id: String) -> Vec<String> {
        let mut domains = self.0.lock();
        if !domain_id.is_empty()
            && domain_id != super::registry::PERSONAL_DOMAIN_ID
        {
            domains.insert(domain_id);
        }
        domains.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::SimpleModeUnlocks;

    #[test]
    fn simple_mode_unlocks_survive_frontend_recreation_but_not_process_restart()
    {
        let runtime = SimpleModeUnlocks::default();
        assert_eq!(runtime.unlock("a".into()), vec!["a"]);
        assert_eq!(runtime.unlock("a".into()), vec!["a"]);
        assert_eq!(runtime.unlock("b".into()), vec!["a", "b"]);
        // A recreated frontend reads the same state owned by the running app.
        assert_eq!(runtime.unlocked_domains(), vec!["a", "b"]);
        assert_eq!(runtime.unlock("personal".into()), vec!["a", "b"]);
        assert_eq!(runtime.unlock(String::new()), vec!["a", "b"]);
        assert!(SimpleModeUnlocks::default().unlocked_domains().is_empty());
    }
}
