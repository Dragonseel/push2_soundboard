use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::PathBuf,
};

use rspotify::{
    model::{AdditionalType, Country, Market},
    prelude::*,
    scopes, AuthCodeSpotify, Config, Credentials, OAuth,
};

use crate::MyError;

pub struct Spotify {
    client: AuthCodeSpotify,
}

impl Spotify {
    pub async fn new() -> Spotify {
        let credentials = Credentials::new(
            "56dea31bdc754cda884e236084e20901",
            "3e21efc2285f496786acd88f4f888e65",
        );
        println!("Created Credentials");

        let oauth = OAuth {
            redirect_uri: "http://127.0.0.1:8888".into(),
            scopes: scopes!(
                "user-read-recently-played",
                "user-read-currently-playing",
                "user-modify-playback-state",
                "playlist-read-private",
                "playlist-read-collaborative"
            ),
            ..Default::default()
        };

        println!("Created OAuth Default");

        let config = Config {
            token_cached: true,
            cache_path: PathBuf::from("./test.cache"),
            ..Default::default()
        };

        let client = AuthCodeSpotify::with_config(credentials, oauth, config);
        println!("Created client.");

        let client_result = client.get_authorize_url(true);

        match client_result {
            Ok(token) => {
                Spotify::prompt_for_token(&token, &client).await;
            }
            Err(err) => println!("{:?}", err),
        }

        Spotify { client }
    }

    async fn prompt_for_token(url: &str, client: &AuthCodeSpotify) -> Result<(), MyError> {
        match client.read_token_cache(true).await {
            Ok(Some(new_token)) => {
                let expired = new_token.is_expired();

                // Load token into client regardless of whether it's expired o
                // not, since it will be refreshed later anyway.
                *{
                    match client.get_token().lock().await {
                        Ok(val) => val,
                        Err(_) => return Err(MyError::MutexError("Spotify Client")),
                    }
                } = Some(new_token);

                if expired {
                    // Ensure that we actually got a token from the refetch
                    match client.refetch_token().await? {
                        Some(refreshed_token) => {
                            println!("Successfully refreshed expired token from token cache");
                            *{
                                match client.get_token().lock().await {
                                    Ok(val) => val,
                                    Err(_) => return Err(MyError::MutexError("Spotify Client")),
                                }
                            } = Some(refreshed_token)
                        }
                        // If not, prompt the user for it
                        None => {
                            println!("Unable to refresh expired token from token cache");
                            let code =
                                Spotify::get_code_from_user(url, &client.get_oauth().state).await?;
                            client.request_token(&code).await?;
                        }
                    }
                }
            }
            // Otherwise following the usual procedure to get the token.
            _ => {
                let code = Spotify::get_code_from_user(url, &client.get_oauth().state).await?;
                client.request_token(&code).await?;
            }
        }

        client.write_token_cache().await?;

        Ok(())
    }

    pub async fn get_code_from_user(url: &str, expected_state: &str) -> Result<String, MyError> {
        open::that(url).expect("Could not open browser");

        let tcp_listener = TcpListener::bind("127.0.0.1:8888")?;

        let mut code = String::new();
        let mut state = String::new();

        if let Ok((mut stream, _socketaddr)) = tcp_listener.accept() {
            let buf_reader = BufReader::new(&mut stream);
            let http_request: Vec<_> = buf_reader
                .lines()
                .map(|result| result.expect("Result of SpotifyRequest was error."))
                .take_while(|line| !line.is_empty())
                .collect();

            for line in http_request {
                if line.starts_with("GET /?code=") {
                    let parts: Vec<&str> = line.split("&").collect();

                    code = String::from(&(parts[0])[11..]);
                    state = String::from(&(parts[1])[6..(parts[1].len() - 9)]);

                    println!("{:?}", code);
                    println!("{:?}", state);
                    break;
                }
            }

            let status_line = "HTTP/1.1 200 OK";
            let contents = "<!DOCTYPE html>
            <html lang=\"en\">
              <head>
                <meta charset=\"utf-8\">
                <title>Please close!</title>
              </head>
              <body>
                <h1>Authentication continues in app!</h1>
                <p>You can close this browser page.</p>
              </body>
            </html>";
            let length = contents.len();

            let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

            stream.write_all(response.as_bytes())?;
        }

        // Making sure the state is the same

        if &state != expected_state {
            println!("Request state doesn't match the callback state");
        }

        Ok(code)
    }

    pub async fn test(&mut self) {
        println!("Query playing");
        //let playing =self.client.current_playing(None, Some([&AdditionalType::Track])).await.unwrap();

        let market = Market::Country(Country::Germany);
        let additional_types = [AdditionalType::Episode];
        let artists = self
            .client
            .current_playing(Some(market), Some(&additional_types))
            .await;

        println!("Response: {artists:#?}");
    }

    pub async fn get_current_song(&self) -> String {
        let market = Market::Country(Country::Germany);
        let additional_types = [AdditionalType::Track];
        let artists = self
            .client
            .current_playing(Some(market), Some(&additional_types))
            .await;

        if let Ok(Some(context)) = artists {
            if let Some(item) = context.item {
                match item {
                    rspotify::model::PlayableItem::Track(track) => {
                        let mut output = String::new();

                        for artist in track.artists {
                            if !output.is_empty() {
                                output += ", ";
                            }
                            output += &artist.name;
                        }

                        output += " - ";
                        output += &track.name;

                        return output;
                    }
                    rspotify::model::PlayableItem::Episode(episode) => {
                        return "Unsupported Episode thing.".into();
                    }
                }
            } else {
                return "None".into();
            }
        } else {
            return "None".into();
        }
    }

    pub async fn play(&self) {
        self.client.resume_playback(None, None).await;
    }

    pub async fn pause(&self) {
        self.client.pause_playback(None).await;
    }
}
