use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::PathBuf,
};

use rspotify::{
    model::{AdditionalType, Country, Market, PrivateUser, SimplifiedPlaylist},
    prelude::*,
    scopes, AuthCodeSpotify, ClientError, Config, Credentials, OAuth,
};

use crate::MyError;

pub struct Spotify {
    client: AuthCodeSpotify,
    user: PrivateUser,
}

impl Spotify {
    pub fn new() -> Result<Spotify, MyError> {
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
                Spotify::prompt_for_token(&token, &client)?;
            }
            Err(err) => println!("{:?}", err),
        }

        let current_user = match client.current_user() {
            Ok(user) => user,
            Err(err) => {
                println!("{:?}", err);
                return Err(MyError::SpotifyDetailedError("Could not get current user"));
            }
        };

        Ok(Spotify {
            client,
            user: current_user,
        })
    }

    fn prompt_for_token(url: &str, client: &AuthCodeSpotify) -> Result<(), MyError> {
        match client.read_token_cache(true) {
            Ok(Some(new_token)) => {
                let expired = new_token.is_expired();

                // Load token into client regardless of whether it's expired o
                // not, since it will be refreshed later anyway.
                *{
                    match client.get_token().lock() {
                        Ok(val) => val,
                        Err(_) => return Err(MyError::MutexError("Spotify Client")),
                    }
                } = Some(new_token);

                if expired {
                    // Ensure that we actually got a token from the refetch
                    match client.refetch_token()? {
                        Some(refreshed_token) => {
                            println!("Successfully refreshed expired token from token cache");
                            *{
                                match client.get_token().lock() {
                                    Ok(val) => val,
                                    Err(_) => return Err(MyError::MutexError("Spotify Client")),
                                }
                            } = Some(refreshed_token)
                        }
                        // If not, prompt the user for it
                        None => {
                            println!("Unable to refresh expired token from token cache");
                            let code = Spotify::get_code_from_user(url, &client.get_oauth().state)?;
                            client.request_token(&code)?;
                        }
                    }
                }
            }
            // Otherwise following the usual procedure to get the token.
            _ => {
                let code = Spotify::get_code_from_user(url, &client.get_oauth().state)?;
                client.request_token(&code)?;
            }
        }

        client.write_token_cache()?;

        Ok(())
    }

    pub fn get_code_from_user(url: &str, expected_state: &str) -> Result<String, MyError> {
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

    pub fn get_current_song(&self) -> String {
        let market = Market::Country(Country::Germany);
        let additional_types = [AdditionalType::Track];
        let artists = self
            .client
            .current_playing(Some(market), Some(&additional_types));

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
                    rspotify::model::PlayableItem::Episode(_episode) => {
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

    pub fn play(&self) -> Result<(), MyError> {
        let result = self.client.resume_playback(None, None);

        // We want to filter the 403 status-code response out, it is technically an error,
        // but only because we wanted to pause on a already paused song.
        match result {
            Ok(_) => return Ok(()),
            Err(error) => match error {
                ClientError::ParseJson(_) => {
                    return Err(MyError::SpotifyDetailedError("ParseJson"))
                }
                ClientError::ParseUrl(_) => return Err(MyError::SpotifyDetailedError("ParseUrl")),
                ClientError::Http(ureq_err) => match *ureq_err {
                    rspotify::http::HttpError::Transport(_) => {
                        return Err(MyError::SpotifyDetailedError("UreqTransport"))
                    }
                    rspotify::http::HttpError::Io(_) => {
                        return Err(MyError::SpotifyDetailedError("UreqIo"))
                    }
                    rspotify::http::HttpError::StatusCode(code) => match code.status() {
                        403 => return Ok(()),
                        _val => return Err(MyError::SpotifyDetailedError("UreqHttpCode")),
                    },
                },
                ClientError::Io(_) => return Err(MyError::SpotifyDetailedError("Io")),
                ClientError::CacheFile(_) => {
                    return Err(MyError::SpotifyDetailedError("CacheFile"))
                }
                ClientError::Model(_) => return Err(MyError::SpotifyDetailedError("Model")),
            },
        }
    }

    pub fn pause(&self) -> Result<(), MyError> {
        let result = self.client.pause_playback(None);

        // We want to filter the 403 status-code response out, it is technically an error,
        // but only because we wanted to pause on a already paused song.
        match result {
            Ok(_) => return Ok(()),
            Err(error) => match error {
                ClientError::ParseJson(_) => {
                    return Err(MyError::SpotifyDetailedError("ParseJson"))
                }
                ClientError::ParseUrl(_) => return Err(MyError::SpotifyDetailedError("ParseUrl")),
                ClientError::Http(ureq_err) => match *ureq_err {
                    rspotify::http::HttpError::Transport(_) => {
                        return Err(MyError::SpotifyDetailedError("UreqTransport"))
                    }
                    rspotify::http::HttpError::Io(_) => {
                        return Err(MyError::SpotifyDetailedError("UreqIo"))
                    }
                    rspotify::http::HttpError::StatusCode(code) => match code.status() {
                        403 => return Ok(()),
                        _val => return Err(MyError::SpotifyDetailedError("UreqHttpCode")),
                    },
                },
                ClientError::Io(_) => return Err(MyError::SpotifyDetailedError("Io")),
                ClientError::CacheFile(_) => {
                    return Err(MyError::SpotifyDetailedError("CacheFile"))
                }
                ClientError::Model(_) => return Err(MyError::SpotifyDetailedError("Model")),
            },
        }
    }

    pub fn skip(&self) -> Result<(), MyError> {
        let result = self.client.next_track(None);

        // We want to filter the 403 status-code response out, it is technically an error,
        // but only because we wanted to pause on a already paused song.
        match result {
            Ok(_) => return Ok(()),
            Err(error) => match error {
                ClientError::ParseJson(_) => {
                    return Err(MyError::SpotifyDetailedError("ParseJson"))
                }
                ClientError::ParseUrl(_) => return Err(MyError::SpotifyDetailedError("ParseUrl")),
                ClientError::Http(ureq_err) => match *ureq_err {
                    rspotify::http::HttpError::Transport(_) => {
                        return Err(MyError::SpotifyDetailedError("UreqTransport"))
                    }
                    rspotify::http::HttpError::Io(_) => {
                        return Err(MyError::SpotifyDetailedError("UreqIo"))
                    }
                    rspotify::http::HttpError::StatusCode(code) => match code.status() {
                        403 => return Ok(()),
                        _val => return Err(MyError::SpotifyDetailedError("UreqHttpCode")),
                    },
                },
                ClientError::Io(_) => return Err(MyError::SpotifyDetailedError("Io")),
                ClientError::CacheFile(_) => {
                    return Err(MyError::SpotifyDetailedError("CacheFile"))
                }
                ClientError::Model(_) => return Err(MyError::SpotifyDetailedError("Model")),
            },
        }
    }

    pub fn get_user_playlists(&self) -> Result<Vec<SimplifiedPlaylist>, MyError> {
        let playlists: Vec<Result<rspotify::model::SimplifiedPlaylist, ClientError>> =
            self.client.user_playlists(self.user.id.clone()).collect();

        let playlists = playlists
            .into_iter()
            .map(|playlist_result| playlist_result.expect("Could not get a playlist"))
            .collect();

        Ok(playlists)
    }

    pub fn play_playlist(&self, playlist: SimplifiedPlaylist) -> Result<(), MyError> {
        self.client.start_context_playback(
            PlayContextId::Playlist(playlist.id),
            None,
            None,
            None,
        )?;

        Ok(())
    }
}
