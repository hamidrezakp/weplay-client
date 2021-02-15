use std::fmt::{self, Display};
use std::io::{BufReader, Read};
use std::os::unix::net::UnixStream;

pub struct Mpv {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
    name: String,
}

impl Mpv {
    pub fn connect(socket: &str) -> Result<Mpv, ErrorCode> {
        match UnixStream::connect(socket) {
            Ok(stream) => {
                let cloned_stream = stream.try_clone().expect("cloning UnixStream");
                return Ok(Mpv {
                    stream,
                    reader: BufReader::new(cloned_stream),
                    name: String::from(socket),
                });
            }
            Err(internal_error) => Err(ErrorCode::ConnectError(internal_error.to_string())),
        }
    }

    pub fn disconnect(&self) {
        let mut stream = &self.stream;
        stream
            .shutdown(std::net::Shutdown::Both)
            .expect("socket disconnect");
        let mut buffer = [0; 32];
        for _ in 0..stream.bytes().count() {
            stream.read(&mut buffer[..]).unwrap();
        }
    }

    pub fn get_stream_ref(&self) -> &UnixStream {
        &self.stream
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[derive(Debug)]
pub enum ErrorCode {
    MpvError(String),
    JsonParseError(String),
    ConnectError(String),
    JsonContainsUnexptectedType,
    UnexpectedResult,
    UnexpectedValue,
    UnsupportedType,
    ValueDoesNotContainBool,
    ValueDoesNotContainF64,
    ValueDoesNotContainHashMap,
    ValueDoesNotContainPlaylist,
    ValueDoesNotContainString,
    ValueDoesNotContainUsize,
}

pub enum MpvCommand {
    Pause,
    Play,
    Seek(TimeStamp),
}

pub fn run_mpv_command(instance: &Mpv, command: &str, args: &[&str]) -> Result<(), Error> {
    let mut ipc_string = format!("{{ \"command\": [\"{}\"", command);
    if args.len() > 0 {
        for arg in args {
            ipc_string.push_str(&format!(", \"{}\"", arg));
        }
    }
    ipc_string.push_str("] }\n");
    ipc_string = ipc_string;
    match serde_json::from_str::<Value>(&send_command_sync(instance, &ipc_string)) {
        Ok(feedback) => {
            if let Value::String(ref error) = feedback["error"] {
                if error == "success" {
                    Ok(())
                } else {
                    Err(Error(ErrorCode::MpvError(error.to_string())))
                }
            } else {
                Err(Error(ErrorCode::UnexpectedResult))
            }
        }
        Err(why) => Err(Error(ErrorCode::JsonParseError(why.to_string()))),
    }
}

fn send_command_sync(instance: &Mpv, command: &str) -> String {
    let mut stream = &instance.stream;
    match stream.write_all(command.as_bytes()) {
        Err(why) => panic!("Error: Could not write to socket: {}", why),
        Ok(_) => {
            debug!("Command: {}", command.trim_end());
            let mut response = String::new();
            {
                let mut reader = BufReader::new(stream);
                while !response.contains("\"error\":") {
                    response.clear();
                    reader.read_line(&mut response).unwrap();
                }
            }
            debug!("Response: {}", response.trim_end());
            response
        }
    }
}

fn main() {
    println!("Hello, world!");
}
