//! The layers memory is kept in: a bot's own, a brain's, and nothing else.
//!
//! A brain is memory several bots share without mixing it with their own: a project's, or the one called "Fleet",
//! which is the memory every bot used to share and lives in the folder it always did. A layer is named by the key
//! its files are stored under: a bare bot id is that bot's own, `brain:<id>` is a brain's, and the empty key (no
//! layer, `None` here) is the shared folder.
//!
//! A bot sees its own layer first and its brain's second: what one of them doesn't say, the other may.

use serde::{Deserialize, Serialize};

/// The key prefix of a brain's layer.
pub const BRAIN_PREFIX: &str = "brain:";

/// The layer a change goes to, as a dreaming run names it in its answer.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// The bot's own memory.
    #[default]
    Personal,
    /// The memory of the brain the bot works in.
    Brain,
}

impl Scope {
    pub fn name(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Brain => "brain",
        }
    }
}

/// A brain: memory the bots working in it share.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brain {
    pub id: String,
    pub name: String,
    /// The key its files are stored under; `None` for the Fleet brain, which keeps the shared folder memory always
    /// had.
    pub layer: Option<String>,
}

impl Brain {
    /// A project's brain, in a folder of its own under `brains/<id>`.
    pub fn project(id: &str, name: &str) -> Self {
        Self { id: id.to_owned(), name: name.to_owned(), layer: Some(format!("{BRAIN_PREFIX}{id}")) }
    }

    /// The Fleet brain, which is the shared folder.
    pub fn fleet(id: &str, name: &str) -> Self {
        Self { id: id.to_owned(), name: name.to_owned(), layer: None }
    }
}

/// The layers a bot sees.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layers {
    /// The bot whose own layer it is.
    pub bot: String,
    /// The brain it works in.
    pub brain: Option<Brain>,
}

impl Layers {
    /// A bot with no brain: only its own memory.
    pub fn of(bot: &str) -> Self {
        Self { bot: bot.to_owned(), brain: None }
    }

    pub fn with_brain(bot: &str, brain: Brain) -> Self {
        Self { bot: bot.to_owned(), brain: Some(brain) }
    }

    /// Whether the bot sees the layer of `scope`.
    pub fn sees(&self, scope: Scope) -> bool {
        match scope {
            Scope::Personal => true,
            Scope::Brain => self.brain.is_some(),
        }
    }

    /// The key `scope` is stored under (`None`: the shared folder), when the bot sees it. A brain without a layer is
    /// the Fleet one, so check [`sees`](Self::sees) to tell it from a bot without a brain.
    pub fn layer(&self, scope: Scope) -> Option<&str> {
        match scope {
            Scope::Personal => Some(&self.bot),
            Scope::Brain => self.brain.as_ref()?.layer.as_deref(),
        }
    }

    /// The layers seen, most specific first.
    pub fn keys(&self) -> impl Iterator<Item = (Scope, Option<&str>)> {
        let own = Some((Scope::Personal, Some(self.bot.as_str())));
        own.into_iter().chain(self.sees(Scope::Brain).then(|| (Scope::Brain, self.layer(Scope::Brain))))
    }

    /// The name of the brain the bot works in.
    pub fn brain_name(&self) -> Option<&str> {
        self.brain.as_ref().map(|brain| brain.name.as_str())
    }

    /// The scope a layer key belongs to, `None` when the bot doesn't see it.
    pub fn scope_of(&self, layer: Option<&str>) -> Option<Scope> {
        self.keys().find(|(_, key)| *key == layer).map(|(scope, _)| scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bot_sees_its_own_layer_then_its_brains() {
        let alone = Layers::of("a");
        assert_eq!(alone.keys().collect::<Vec<_>>(), [(Scope::Personal, Some("a"))]);
        let project = Layers::with_brain("a", Brain::project("p", "Weather"));
        assert_eq!(project.keys().collect::<Vec<_>>(), [(Scope::Personal, Some("a")), (Scope::Brain, Some("brain:p"))]);
        assert_eq!(project.scope_of(Some("brain:p")), Some(Scope::Brain));
        // The Fleet brain keeps the shared folder, so its layer is the empty key.
        let fleet = Layers::with_brain("a", Brain::fleet("f", "Fleet"));
        assert!(fleet.sees(Scope::Brain) && fleet.layer(Scope::Brain).is_none());
        assert_eq!(fleet.keys().count(), 2);
        assert_eq!(fleet.scope_of(None), Some(Scope::Brain));
        assert!(alone.layer(Scope::Brain).is_none() && !alone.sees(Scope::Brain));
    }
}
