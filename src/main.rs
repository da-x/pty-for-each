extern crate libc;
extern crate errno;
extern crate termion;
extern crate structopt;
extern crate regex;

#[macro_use]
extern crate structopt_derive;

use std::io::BufReader;
use std::io::BufRead;
use structopt::StructOpt;
use structopt::clap::AppSettings;
use std::thread;
use std::collections::HashMap;
use std::sync::mpsc::{sync_channel, Receiver};
use regex::Regex;

/// Process ID of child process
///
/// Necessary to put this in static storage for `sigchld` to have access
macro_rules! die {
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
        ::std::process::exit(1);
    }}
}

mod tty;

/// Options for pty-for-each
#[derive(StructOpt, Debug)]
#[structopt(global_settings_raw = "&[AppSettings::ColoredHelp, AppSettings::VersionlessSubcommands]")]
enum Opt {
    #[structopt(name = "key")]
    /// Execute by key
    Key {
        /// Name to use for the key for interpolation, values are used as prefixes
        name: String,

        /// Only show commands, don't execute them
        #[structopt(short = "d", long = "dry-run")]
        dryrun: bool,

        /// Terminal line length if not inheriting from current terminal
        #[structopt(short = "c", long = "columns", default_value = "80")]
        columns: u16,

        /// Command to execute
        #[structopt(name = "VALUES_AND_COMMAND")]
        values_and_command: Vec<String>,
    },

    #[structopt(name = "single")]
    /// Just run a single command without any interpolation
    Single {
        /// Prefix to use
        prefix: String,

        /// Only show commands, don't execute them
        #[structopt(short = "d", long = "dry-run")]
        dryrun: bool,

        /// Terminal line length if not inheriting from current terminal
        #[structopt(short = "c", long = "columns", default_value = "80")]
        columns: u16,

        /// Command to execute
        #[structopt(name = "COMMAND")]
        command: Vec<String>,
    }
}

enum Message {
    Line(u32, String),
    Terminated(u32),
}

struct Subprogram {
    pty: tty::Pty,
    prefix: String,
}

struct Interpolator(String, Regex, Regex);

impl Interpolator {
    ///
    /// This struct manages regexes for replacing %key with a given value,
    /// allowing for \%key to be escaped back to %key.
    ///
    fn new(key: &String) -> Self {
        let escaped = regex::escape(key);
        let re1_str = format!("^%{}|(?P<c>[^\\\\])%{}", escaped, escaped);
        let re2_str = format!("\\\\%{}", escaped);
        let re1 = Regex::new(re1_str.as_str()).unwrap();
        let re2 = Regex::new(re2_str.as_str()).unwrap();

        Interpolator(format!("%{}", key), re1, re2)
    }

    fn interpolate(&self, text: &String, value: &String) -> String {
        let t = String::from("${c}") + value;
        let v = String::from(self.1.replace_all(text, t.as_str()));
        String::from(self.2.replace_all(v.as_str(), self.0.as_str()))
    }
}


fn make_programs(opt: &Opt) -> HashMap<u32, Subprogram> {
    match opt {
        &Opt::Key { ref name, dryrun, columns, ref values_and_command } => {
            let name_interpolator = Interpolator::new(name);
            let mut programs = HashMap::new();
            let splits : Vec<_> = values_and_command.split(|param| param == "%%").collect();
            if splits.len() != 2 {
                die!("Expected '%%' separator between keys and command");
            }
            let values = &splits[0];
            let command = &splits[1];
            if command.len() == 0 {
                die!("Expected non empty command");
            }

            let dims = match termion::terminal_size() {
                Ok(r) => r,
                Err(_) => (25, columns),
            };

            for value in values.iter() {
                let prefix = format!("{}: ", value);
                let dims_sans_prefix = (dims.0,
                        std::cmp::max(1, dims.1 - prefix.len() as u16));
                let subcommand : Vec<_> = command.iter().map(
                    |x| name_interpolator.interpolate(x, value)).collect();

                if dryrun {
                    // Shell escaping for parameters that need it
                    println!("{}", subcommand.join(" "));
                } else {
                    let tty = tty::new(&subcommand, dims_sans_prefix).unwrap();
                    programs.insert(tty.key(), Subprogram { pty: tty, prefix });
                }
            }

            programs
        },
        &Opt::Single { ref prefix, dryrun, columns, ref command } => {
            let mut programs = HashMap::new();
            if command.len() == 0 {
                die!("Expected non empty command");
            }

            let dims = match termion::terminal_size() {
                Ok(r) => r,
                Err(_) => (25, columns),
            };

            let prefix_delim = if prefix.len() > 0 { 
                format!("{}: ", prefix)
            } else {
                prefix.clone()
            };

            let dims_sans_prefix = (dims.0,
                                    std::cmp::max(1, dims.1 - prefix.len() as u16));
            if dryrun {
                // Shell escaping for parameters that need it
                println!("{}", command.join(" "));
            } else {
                let tty = tty::new(&command, dims_sans_prefix).unwrap();
                programs.insert(tty.key(), Subprogram { pty: tty, prefix: prefix_delim });
            }

            programs
        }
    }
}

type ThreadsMap = HashMap<u32, thread::JoinHandle<()>>;

fn handle_programs(programs: &HashMap<u32, Subprogram>) -> (ThreadsMap, Receiver<Message>)
{
    let mut threads = HashMap::new();
    let (sender, receiver) = sync_channel(0x1000);

    for (key, program) in programs.iter() {
        let sender = sender.clone();
        let key = *key;
        let file = program.pty.reader();

        let child = thread::spawn(move || {
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            loop {
                match reader.read_line(&mut line) {
                    Ok(_) => {
                        sender.send(Message::Line(key, line.clone())).unwrap();
                        line.clear();
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
            sender.send(Message::Terminated(key)).unwrap();
        });

        threads.insert(key, child);
    }

    (threads, receiver)
}

fn handle_mainloop(mut programs: HashMap<u32, Subprogram>,
                   mut threads: ThreadsMap,
                   receiver: Receiver<Message>)
{
    while programs.len() > 0 {
        let msg = receiver.recv().unwrap();
        match msg {
            Message::Line(key, line) => {
                let program = programs.get(&key).unwrap();
                print!("{}{}", program.prefix, line);
            }
            Message::Terminated(key) => {
                programs.remove(&key);
                let thread = threads.remove(&key).unwrap();
                thread.join().expect("join error");
            }
        }
    }
}

fn main() {
    let opt = Opt::from_args();
    let programs = make_programs(&opt);
    let (threads, receiver) = handle_programs(&programs);
    handle_mainloop(programs, threads, receiver);
}
