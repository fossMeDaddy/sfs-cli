use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use tiny_http::Server;

use crate::utils::net::get_local_addr;
use crate::{shared_types::CliSubCmd, utils::paths::get_paths_from_pattern};

#[derive(Parser)]
pub struct ServeCommand {
    /// list of path patterns e.g. "/myprivatedir/**/img_*.{jpeg|jpg|png|webp|pdf}"
    patterns: Vec<String>,
}

impl CliSubCmd for ServeCommand {
    async fn run(&self) {
        let mut paths = self
            .patterns
            .iter()
            .flat_map(|patt| get_paths_from_pattern(patt).unwrap())
            .collect::<Vec<PathBuf>>();

        paths.sort();
        paths.dedup();

        println!("Files selected: {}", paths.len());

        let local_addr = get_local_addr().unwrap();

        let server = Server::http(format!("{}:{}", local_addr.0, local_addr.1)).unwrap();
        println!("Server started at http://{}:{}", local_addr.0, local_addr.1);

        loop {
            let request = server.recv().unwrap();

            if request.url() == "/" {
                let mut html = String::from("<ol>");
                for path in &paths {
                    let path_str = path.to_str().unwrap();
                    html += &format!(
                        "<li><a href=\"/file/{}\">{}</a></li>",
                        urlencoding::encode(path_str),
                        path_str,
                    );
                }
                html += "</ol>";

                let mut response = tiny_http::Response::from_string(html).with_status_code(200);
                response.add_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap(),
                );
                request.respond(response).unwrap();
            } else if request.url().starts_with("/file/") {
                let pathname = request.url().replace("/file/", "");
                let pathname = match urlencoding::decode(pathname.as_str()) {
                    Ok(pathname) => pathname.to_string(),
                    Err(err) => {
                        let response = tiny_http::Response::from_string("Malformed file name")
                            .with_status_code(400);
                        let _ = request.respond(response);
                        println!("{:?}", err);
                        continue;
                    }
                };

                let file_data = match fs::read(Path::new(&pathname)) {
                    Ok(data) => data,
                    Err(e) => {
                        request
                            .respond(
                                tiny_http::Response::from_string("File not found")
                                    .with_status_code(404),
                            )
                            .unwrap();

                        println!("Error reading file: {}", e);
                        continue;
                    }
                };

                let mut response = tiny_http::Response::from_data(file_data);
                response.add_header(
                    tiny_http::Header::from_bytes(
                        &b"Content-Type"[..],
                        &b"application/octet-stream"[..],
                    )
                    .unwrap(),
                );
                request.respond(response).unwrap();
            } else {
                request
                    .respond(
                        tiny_http::Response::from_string("Requested path not found")
                            .with_status_code(404),
                    )
                    .unwrap();
            }
        }
    }
}
