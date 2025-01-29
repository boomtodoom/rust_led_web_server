use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream){
    let buf_reader = BufReader::new(&stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let (status_line, filename) = if request_line.starts_with("GET / ") || request_line.starts_with("GET /home ") {
        ("HTTP/1.1 200 OK", "home.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "404.html")
    };

    let path = format!("src/html/{}", filename);

    let contents = fs::read_to_string(path).unwrap();
    let length = contents.len();

    let response = 
        format!("{}\r\nContent-Length: {}\r\n\r\n{}", status_line, length, contents);

    stream.write_all(response.as_bytes()).unwrap();

}

