#![allow(non_camel_case_types)]
use punter::PunterId;
use punter::SiteId;
use punter::Punter;
use punter::Input;

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
pub enum TurnS {
    // move is a reserved keyword
    #[serde(rename = "move")]
    turn { moves: Vec<Move> },

    stop {
        moves: Vec<Move>,
        scores: Vec<Score>,
    },

    timeout ( f64 ),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Move {
    claim (Claim),

    pass (Pass),

    splurge (Splurge),

    option (Claim),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Score {
    punter: PunterId,
    score: isize,
}




#[derive(Serialize, Deserialize, Debug)]
pub struct OfflineReadyP {
    pub ready: PunterId,
    pub state: Punter,
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum OfflineInput {
    Setup (Input),

    Turn (OfflineTurn),

    Stop (OfflineStop),

    Timeout {
        timeout: f64,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfflineTurn {
    // move is a reserved keyword
    #[serde(rename = "move")]
    pub turn: Moves,
    pub state: Punter,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Moves {
    pub moves: Vec<Move>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfflineStop {
    pub stop: MovesScores,
    pub state: Punter,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MovesScores {
    pub moves: Vec<Move>,
    pub scores: Vec<Score>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum OfflineMove {
    Claim (OfflineClaim),

    Pass (OfflinePass),

    Splurge (OfflineSplurge),

    Option (OfflineOption),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfflineClaim {
    pub claim: Claim,
    pub state: Punter,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Claim {
    pub punter: PunterId,
    pub source: SiteId,
    pub target: SiteId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfflinePass {
    pub pass: Pass,
    pub state: Punter,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Pass {
    pub punter: PunterId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfflineSplurge {
    pub splurge: Splurge,
    pub state: Punter,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Splurge {
    pub punter: PunterId,
    pub route: Vec<SiteId>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfflineOption {
    pub option: Claim,
    pub state: Punter,
}
