use std::collections::HashSet;

use super::protocol;

pub type PunterId = usize;
pub type SiteId = usize;

pub struct State {
    punter_id: PunterId,
    punters: PunterId,
    sites: HashSet<SiteId>,
    rivers: Vec<River>,
    mines: HashSet<SiteId>,
}

pub struct River {
    source: SiteId,
    target: SiteId,
    owner: Option<PunterId>,
}

impl River {
    // Use this to build new River structures
    pub fn new(source: SiteId, target: SiteId) -> River {
        River {
            source: source,
            target: target,
            owner:  None
        }
    }

    // Set the owner to a punter
    pub fn set_owner(&mut self, owner_id: PunterId) {
        self.owner = Some(owner_id)
    }
}

pub fn handshake() -> protocol::HandshakeP {
    protocol::HandshakeP {
        me: String::from("test"),
    }
}
