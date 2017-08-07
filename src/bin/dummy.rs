extern crate getopts;
extern crate bufstream;
extern crate serde;
extern crate serde_json;
extern crate rand;

use std::time::Instant;

extern crate punter;
use punter::protocol;
use punter::punter::PunterType;
use punter::punter::Punter;

fn main() {
    let setup = serde_json::from_str("{\"punter\":1,\"punters\":2,\"map\":{\"sites\":[{\"id\":4},{\"id\":0},{\"id\":1},{\"id\":7},{\"id\":6},{\"id\":5},{\"id\":3},{\"id\":2}],\"rivers\":[{\"source\":3,\"target\":4,\"owner\":null},{\"source\":0,\"target\":1,\"owner\":null},{\"source\":2,\"target\":3,\"owner\":null},{\"source\":1,\"target\":3,\"owner\":null},{\"source\":5,\"target\":6,\"owner\":null},{\"source\":4,\"target\":5,\"owner\":null},{\"source\":3,\"target\":5,\"owner\":null},{\"source\":6,\"target\":7,\"owner\":null},{\"source\":5,\"target\":7,\"owner\":null},{\"source\":1,\"target\":7,\"owner\":null},{\"source\":0,\"target\":7,\"owner\":null},{\"source\":1,\"target\":2,\"owner\":null}],\"mines\":[1,5]}}").unwrap();
    let mut punter = Punter::new(setup, PunterType::MCTS);
    let turn: protocol::TurnS = serde_json::from_str("{\"move\":{\"moves\":[{\"claim\":{\"punter\":0,\"source\":3,\"target\":5}},{\"pass\":{\"punter\":1}}]}}")
        .expect("Could not parse turn");
    println!("{:?}", turn);
    if let protocol::TurnS::stop{scores, moves: _} = turn {
        println!("Done with game. Scores: {:?}", scores);
        return;
    }

    if let protocol::TurnS::turn {moves} = turn {
        punter.process_turn(&moves);
        let next_move = punter.make_move(Instant::now(), 1);
        println!("{:?}", next_move);
    }
}
