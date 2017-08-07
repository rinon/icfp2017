#[macro_use]
extern crate serde_derive;

extern crate getopts;
extern crate bufstream;
extern crate serde;
extern crate serde_json;
extern crate rand;

use getopts::Options;
use std::env;
use std::net::TcpStream;
use bufstream::BufStream;
use std::io::{Read, Write, BufRead};
use std::time::Instant;

mod punter;
mod protocol;
use punter::Punter;

const DEFAULT_SERVER: &str = "punter.inf.ed.ac.uk";
const DEFAULT_PORT: &str = "9001";
const DEFAULT_NAME: &str = "random hackers";
const DEFAULT_TIMEOUT: &str = "1";

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    println!("{}", opts.usage(&brief));
}

fn send_message<T: ?Sized>(stream: &mut BufStream<TcpStream>, msg: &T)
    where
    T: serde::Serialize,
{
    let msg_str = serde_json::to_string(msg).expect("Could not encode message as JSON");

    let _ = stream.write_all(format!("{}:{}", msg_str.len(), msg_str).as_bytes());
    stream.flush().unwrap();
}

fn recv_message<T>(stream: &mut BufStream<TcpStream>) -> Result<T, serde_json::Error>
    where T: serde::de::DeserializeOwned
{
    let mut buf = vec![];
    let _ = stream.read_until(':' as u8, &mut buf);
    buf.pop(); // Drop colon
    let len = String::from_utf8(buf).unwrap()
        .parse::<u64>().unwrap();
    serde_json::from_reader(stream.take(len))
}

fn online_handshake(stream: &mut BufStream<TcpStream>, name: String) {
    send_message(stream, &punter::handshake(name));
    let handshake: protocol::HandshakeS = recv_message(stream)
        .expect("Could not parse handshake response");
    println!("Registered as: {}", handshake.you);
}

fn online_game_loop(stream: &mut BufStream<TcpStream>, timeout: u8) {
    let setup_begin = Instant::now();
    let setup_input: punter::Input = recv_message(stream)
        .expect("Could not parse setup message");

    let mut punter = Punter::new(setup_input, punter::PunterType::MCTS);
    println!("We are player {}", punter.id());

    let ready_msg = protocol::ReadyP {
        ready: punter.id(),
    };
    send_message(stream, &ready_msg);
    let setup_time = setup_begin.elapsed();
    println!("Setup took {}.{:09}s", setup_time.as_secs(), setup_time.subsec_nanos());

    loop {
        let turn_begin = Instant::now();
        let turn: protocol::TurnS = recv_message(stream)
            .expect("Could not parse turn");
        if let protocol::TurnS::timeout (_) = turn {
            println!("Timout!");
            continue;
        };
        println!("{:?}", turn);
        if let protocol::TurnS::stop{scores, moves: _} = turn {
            println!("Done with game. Scores: {:?}", scores);
            break;
        }

        punter.process_turn(turn);
        let next_move = punter.make_move(turn_begin, timeout);
        println!("{:?}", next_move);
        send_message(stream, &next_move);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("s", "server", "server address", "ADDRESS");
    opts.optopt("p", "port", "port", "PORT");
    opts.optopt("n", "name", "AI name", "NAME");
    opts.optopt("t", "timeout", "Move timeout", "TIMEOUT");
    opts.optflag("h", "help", "print this help menu");
    let matches = opts.parse(&args[1..]).unwrap();
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let server = matches.opt_str("server").unwrap_or(DEFAULT_SERVER.to_string());
    let port: u16 = matches.opt_str("port").unwrap_or(DEFAULT_PORT.to_string())
        .parse().unwrap();
    let name = matches.opt_str("name").unwrap_or(DEFAULT_NAME.to_string());
    let timeout: u8 = matches.opt_str("timeout").unwrap_or(DEFAULT_TIMEOUT.to_string())
        .parse().unwrap();

    let connection = TcpStream::connect((&server[..], port))
        .expect("Connection refused!");
    let mut stream = BufStream::new(connection);

    online_handshake(&mut stream, name);
    online_game_loop(&mut stream, timeout);
}
