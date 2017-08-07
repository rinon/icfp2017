extern crate getopts;
extern crate bufstream;
extern crate serde;
extern crate serde_json;
extern crate rand;

use std::io::{self, Read, Write, BufRead};
use std::time::Instant;

extern crate punter as p;
use p::protocol;
use p::punter::Punter;
use p::punter;

const NAME: &str = "random hackers";
const TIMEOUT: u8 = 1;

struct OfflineGame;

impl OfflineGame {
    fn run(&mut self) {
        self.handshake(String::from(NAME));
        self.game_step(TIMEOUT);
    }

    fn send_message<T: ?Sized>(&mut self, msg: &T)
        where T: serde::Serialize,
    {
        let mut writer = io::stdout();
        let msg_str = serde_json::to_string(msg).expect("Could not encode message as JSON");

        let _ = writer.write_all(format!("{}:{}", msg_str.len(), msg_str).as_bytes());
        writer.flush().unwrap();
    }

    // This requires the reader param because the reader needs to stay in scope
    // until the result is handled
    fn recv_message<T>(&mut self, mut reader: io::StdinLock) -> Result<T, serde_json::Error>
        where T: serde::de::DeserializeOwned
    {
        let mut buf = vec![];
        let _ = reader.read_until(':' as u8, &mut buf);
        buf.pop(); // Drop colon
        let len = String::from_utf8(buf).unwrap()
            .parse::<u64>().unwrap();
        serde_json::from_reader(reader.take(len))
    }

    fn handshake(&mut self, name: String) {
        let stdin = io::stdin();
        let reader = stdin.lock();
        self.send_message(&punter::handshake(name));
        let handshake: protocol::HandshakeS = self.recv_message(reader)
            .expect("Could not parse handshake response");
        // eprintln!("Registered as: {}", handshake.you);
    }

    fn game_step(&mut self, timeout: u8) {
        let time_begin = Instant::now();
        let stdin = io::stdin();
        let setup_input: protocol::OfflineInput = self.recv_message(stdin.lock())
            .expect("Could not parse offline input message");

        match setup_input {
            protocol::OfflineInput::Setup (setup_input) => {
                let punter = Punter::new(setup_input, punter::PunterType::MCTS);
                // eprintln!("We are player {}", punter.id());

                let ready_msg = protocol::OfflineReadyP {
                    ready: punter.id(),
                    state: punter,
                };
                self.send_message(&ready_msg);
            }
            protocol::OfflineInput::Turn (
                protocol::OfflineTurn {turn, mut state}
            ) => {
                state.process_turn(&turn.moves);
                let next_move = state.make_move(time_begin, timeout);
                match next_move {
                    protocol::Move::claim (claim) =>
                        self.send_message(&protocol::OfflineClaim {
                            claim: claim,
                            state: state,
                        }),
                    protocol::Move::pass (pass) =>
                        self.send_message(&protocol::OfflinePass {
                            pass: pass,
                            state: state,
                        }),
                    protocol::Move::splurge (splurge) =>
                        self.send_message(&protocol::OfflineSplurge {
                            splurge: splurge,
                            state: state,
                        }),
                    protocol::Move::option (option) =>
                        self.send_message(&protocol::OfflineOption {
                            option: option,
                            state: state,
                        }),
                };
            }
            protocol::OfflineInput::Stop (
                protocol::OfflineStop {stop, state}
            ) => {
                // eprintln!("Done with game. Scores: {:?}", stop.scores);
                // eprintln!("Our score: {:?}", stop.scores[state.id()]);
            }
            protocol::OfflineInput::Timeout {timeout: _} => {
                // eprintln!("Timout!");
            }
        }
    }
}

fn main() {
    OfflineGame.run();
}
