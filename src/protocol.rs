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
    pub ready: PunterId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TurnS {
    // move is a reserved keyword
    #[serde(default, rename = "move")]
    turn: Option<Moves>,
    #[serde(default)]
    stop: Option<Finished>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Moves {
    moves: Vec<Move>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Finished {
    moves: Vec<Move>,
    scores: Vec<Score>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Move {
    #[serde(default)]
    claim: Option<Claim>,
    #[serde(default)]
    pass: Option<Pass>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Claim {
    punter: PunterId,
    source: SiteId,
    target: SiteId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Pass {
    punter: PunterId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Score {
    punter: PunterId,
    score: usize,
}
