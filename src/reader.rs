/// Reader will read the entries from stdin or command output
/// And send the entries to controller, the controller will save it into model.

extern crate libc;

use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::io::{stdin, BufRead, BufReader};
use std::error::Error;
use util::eventbox::EventBox;
use event::Event;
use item::Item;

const READER_EVENT_DURATION: u64 = 30;

pub struct Reader {
    cmd: String, // command to invoke
    eb: Arc<EventBox<Event>>,         // eventbox
    items: Arc<RwLock<Vec<Item>>>, // all items
}

impl Reader {

    pub fn new(cmd: String, eb: Arc<EventBox<Event>>, items: Arc<RwLock<Vec<Item>>>) -> Self {
        Reader{cmd: cmd, eb: eb, items: items}
    }

    // invoke find comand.
    fn get_command_output(&self) -> Result<Box<BufRead>, Box<Error>> {
        let command = try!(Command::new("sh")
                           .arg("-c")
                           .arg(&self.cmd)
                           .stdout(Stdio::piped())
                           .stderr(Stdio::null())
                           .spawn());
        let stdout = try!(command.stdout.ok_or("command output: unwrap failed".to_owned()));
        Ok(Box::new(BufReader::new(stdout)))
    }

    pub fn run(&mut self) {
        // check if the input is TTY
        let istty = unsafe { libc::isatty(libc::STDIN_FILENO as i32) } != 0;

        let mut read;
        if istty {
            read = self.get_command_output().expect("command not found");
        } else {
            read = Box::new(BufReader::new(stdin()))
        };

        loop {
            let mut input = String::new();
            match read.read_line(&mut input) {
                Ok(n) => {
                    if n <= 0 { break; }

                    if input.ends_with("\n") {
                        input.pop();
                        if input.ends_with("\r") {
                            input.pop();
                        }
                    }
                    let mut items = self.items.write().unwrap();
                    items.push(Item::new(input));
                }
                Err(_err) => {} // String not UTF8 or other error, skip.
            }

            self.eb.set_throttle(Event::EvReaderNewItem, Box::new(true), READER_EVENT_DURATION);
        }
        self.eb.set_throttle(Event::EvReaderNewItem, Box::new(false), READER_EVENT_DURATION);
    }
}
