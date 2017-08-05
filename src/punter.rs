use std::collections::HashSet;
use std::collections::HashMap;

use protocol;

pub type PunterId = usize;
pub type SiteId = usize;
pub type RiverId = usize;

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    input: Input,
    owners: HashMap<RiverId, PunterId>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    punter: PunterId,
    punters: PunterId,
    map: InputMap,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InputMap {
    sites: HashSet<Site>,
    rivers: Vec<River>,
    mines: HashSet<SiteId>,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
pub struct Site {
    id: SiteId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct River {
    source: SiteId,
    target: SiteId,
}

pub fn handshake() -> protocol::HandshakeP {
    protocol::HandshakeP {
        me: String::from("test"),
    }
}
