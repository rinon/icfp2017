#[macro_use]
extern crate serde_derive;

extern crate getopts;
extern crate bufstream;
extern crate serde;
extern crate serde_json;
use getopts::Options;
use std::env;
use std::net::TcpStream;
use bufstream::BufStream;
use std::io::Read;
use std::io::Write;
use std::io::BufRead;
mod punter;

const DEFAULT_SERVER: &str = "punter.inf.ed.ac.uk";
const DEFAULT_PORT: &str = "9001";

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    println!("{}", opts.usage(&brief));
}

fn online_game_loop(stream: &mut BufStream<TcpStream>) {
    let mut buf = vec![];
    let msg = serde_json::to_string(&punter::handshake())
        .expect("Could not encode message as JSON");
    println!("{}:{}", msg.len(), msg);
    let _ = stream.write_all(format!("{}:{}", msg.len(), msg).as_bytes());
    stream.flush().unwrap();
    let _ = stream.read_until(':' as u8, &mut buf);
    buf.pop(); // Drop colon
    let len = String::from_utf8(buf.clone()).unwrap()
        .parse::<usize>().unwrap();
    println!("{}", len);
    buf.resize(len, 0);
    stream.read_exact(&mut buf).unwrap();
    println!("{}", String::from_utf8(buf).unwrap());
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
