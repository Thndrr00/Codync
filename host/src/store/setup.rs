use super::Store;
use crate::LockExt;
use anyhow::Result;
impl Store {
    pub fn claim_setup_milestone(&self, name: &str) -> Result<bool> {
        Ok(self
            .db
            .locked()
            .execute("INSERT INTO kv(k,v) VALUES('setup.milestone.' || ?1,'1') ON CONFLICT DO NOTHING", [name])?
            == 1)
    }
}
