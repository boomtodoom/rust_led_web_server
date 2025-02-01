use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    collections::HashMap,
};
use log::{info, error};

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    host: String,
    port: u16,
    static_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: "127.0.0.1".to_string(),
            port: 7878,
            static_dir: "src/html".to_string(),
        }
    }
}

fn load_config() -> Config {
    match fs::read_to_string("config.yaml") {
        Ok(contents) => serde_yaml::from_str(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

fn load_credentials() -> HashMap<String, String> {
    let contents = fs::read_to_string("credentials.txt").expect("Failed to read credentials file");
    let mut credentials = HashMap::new();
    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 2 {
            credentials.insert(parts[0].to_string(), parts[1].to_string());
        }
    }
    credentials
}

fn main() {
    env_logger::init();
    let config = load_config();
    let address = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&address).unwrap();
    info!("Server running on {}", address);

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream, &config);
    }
}

fn handle_connection(mut stream: TcpStream, config: &Config) {
    let buf_reader = BufReader::new(&stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();
    info!("Request: {}", request_line);

    if request_line.starts_with("GET /config ") {
        serve_file(&mut stream, config, "config.html");
    } else if request_line.starts_with("POST /update_config ") {
        let credentials = load_credentials();
        let mut headers = String::new();
        let mut body = String::new();
        let mut in_body = false;

        let buf_reader = BufReader::new(&stream); // Re-create the buf_reader
        for line in buf_reader.lines() {
            let line = line.unwrap();
            if line.is_empty() {
                in_body = true;
                continue;
            }
            if in_body {
                body.push_str(&line);
            } else {
                headers.push_str(&line);
                headers.push('\n');
            }
        }

        info!("Headers: {}", headers);
        info!("Body: {}", body);

        let params: HashMap<_, _> = url::form_urlencoded::parse(body.as_bytes()).into_owned().collect();
        if let (Some(username), Some(password)) = (params.get("username"), params.get("password")) {
            if credentials.get("username") == Some(username) && credentials.get("password") == Some(password) {
                let new_config = Config {
                    host: params.get("host").unwrap_or(&config.host).to_string(),
                    port: params.get("port").unwrap_or(&config.port.to_string()).parse().unwrap_or(config.port),
                    static_dir: params.get("static_dir").unwrap_or(&config.static_dir).to_string(),
                };
                let yaml = serde_yaml::to_string(&new_config).expect("Failed to serialize config");
                fs::write("config.yaml", yaml).expect("Failed to write config file");
                info!("Config updated: {:?}", new_config);
                serve_file(&mut stream, config, "home.html");
            } else {
                error!("Invalid credentials");
                serve_file(&mut stream, config, "404.html");
            }
        } else {
            error!("Missing username or password");
            serve_file(&mut stream, config, "404.html");
        }
    } else {
        let (status_line, filename) = if request_line.starts_with("GET / ") || request_line.starts_with("GET /home ") {
            ("HTTP/1.1 200 OK", "home.html")
        } else {
            ("HTTP/1.1 404 NOT FOUND", "404.html")
        };

        let path = format!("{}/{}", config.static_dir, filename);
        let contents = fs::read_to_string(path).unwrap();
        let length = contents.len();

        let response = format!(
            "{}\r\nContent-Length: {}\r\n\r\n{}", 
            status_line, 
            length, 
            contents
        );

        stream.write_all(response.as_bytes()).unwrap();
    }
}

fn serve_file(stream: &mut TcpStream, config: &Config, filename: &str) {
    let path = format!("{}/{}", config.static_dir, filename);
    let contents = fs::read_to_string(path).unwrap();
    let length = contents.len();

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", 
        length, 
        contents
    );

    stream.write_all(response.as_bytes()).unwrap();
}