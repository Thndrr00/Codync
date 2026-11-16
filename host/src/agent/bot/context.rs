//! Who asked and what set the turn off, for a policy's `actor.*` and `initiator.kind` variables.

use super::Actor;

impl Actor {
    /// `person`, `routine`, `channel` or `network`: what set this turn off.
    ///
    /// A scheduled run says so itself; a message that came over the network carries where it came from;
    /// a lane that isn't the bot's own chat is a group or a channel.
    pub(super) fn initiator(&self) -> &'static str {
        if self.active_routine.is_some() {
            return "routine";
        }
        let origin = self
            .turn_entries
            .iter()
            .find_map(|id| self.hub.store.entry(id).and_then(|entry| entry.data["origin"].as_str().map(str::to_owned)));
        match origin.as_deref() {
            Some("network") => "network",
            Some("channel") => "channel",
            Some(_) => "person",
            None if self.lane.chat == self.cfg.id => "person",
            None => "channel",
        }
    }

    /// The person whose bot this is and their role on this computer; `owner` when it is the owner's.
    pub(super) fn actor(&self) -> (String, String) {
        match &self.cfg.person {
            Some(id) => {
                let role = self
                    .hub
                    .store
                    .person(id)
                    .map_or_else(|| "member".to_owned(), |person| person.role.as_str().to_owned());
                (id.clone(), role)
            }
            None => (String::new(), "owner".to_owned()),
        }
    }
}
