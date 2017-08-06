#![allow(non_camel_case_types)]
use punter::PunterId;
use punter::SiteId;
use punter::Punter;

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
pub struct TurnStateS {
    turn: TurnS,
    state: Punter,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Move {
    claim {
        punter: PunterId,
        source: SiteId,
        target: SiteId,
    },

    pass {
        punter: PunterId,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Score {
    punter: PunterId,
    score: isize,
}
