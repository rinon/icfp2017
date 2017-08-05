use punter::PunterId;
use punter::SiteId;

#[derive(Serialize, Deserialize, Debug)]
pub struct HandshakeP {
    pub me: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HandshakeS {
    pub you: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReadyP {
    ready: PunterId,
}
