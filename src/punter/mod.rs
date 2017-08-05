use std::collections::HashSet;

mod protocol;

type PunterId = usize;
type SiteId = usize;

pub struct State {
    rivers: Vec<River>,
    mines: HashSet<SiteId>,
}

pub struct River {
    source: SiteId,
    target: SiteId,
    owner: Option<PunterId>,
}

pub fn handshake() -> protocol::HandshakeP {
    protocol::HandshakeP {
        me: String::from("test"),
    }
}
