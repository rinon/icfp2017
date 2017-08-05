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
use std::io::Read;
use std::io::Write;
use std::io::BufRead;
mod punter;
mod protocol;
use punter::Punter;

const DEFAULT_SERVER: &str = "punter.inf.ed.ac.uk";
const DEFAULT_PORT: &str = "9001";

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

fn online_game_loop(stream: &mut BufStream<TcpStream>) {
    send_message(stream, &punter::handshake());
    let handshake: protocol::HandshakeS = recv_message(stream)
        .expect("Could not parse handshake response");
    println!("Received name back: {}", handshake.you);

    let setup_input: punter::Input = recv_message(stream)
        .expect("Could not parse setup message");

    let mut punter = Punter::new(setup_input);
    let ready_msg = protocol::ReadyP {
        ready: punter.ready(),
    };
    send_message(stream, &ready_msg);

    while true {
        let turn: protocol::TurnS = recv_message(stream)
            .expect("Could not parse turn");
        println!("{:?}", turn);
        if let protocol::TurnS::stop{moves, scores} = turn {
            println!("Done with game. Scores: {:?}", scores);
            break;
        }

        punter.process_turn(turn);
        let next_move = punter.make_move();
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
    opts.optflag("h", "help", "print this help menu");
    let matches = opts.parse(&args[1..]).unwrap();
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    println!("parsing args...");

    let server = matches.opt_str("server").unwrap_or(DEFAULT_SERVER.to_string());
    let port: u16 = matches.opt_str("port").unwrap_or(DEFAULT_PORT.to_string())
        .parse().unwrap();

    println!("connecting...");
    let mut stream = BufStream::new(TcpStream::connect((&server[..], port)).unwrap());
    println!("connected");

    online_game_loop(&mut stream);
}
