use std::{collections::HashMap, io, sync::Arc};
use axum::{extract::{Path, State}, http::StatusCode, response::{IntoResponse, Response}, routing::{get, post}, Json, Router};
use log::debug;
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
struct Movie {
    pub id: String,
    pub name: String,
    pub year: u16,
    pub was_good: bool
}

struct MoviesState { 
    pub movies: HashMap<String, Movie>,
}

impl MoviesState { 
    fn new() -> MoviesState { 
        MoviesState { 
            movies: HashMap::new(),
        }
    }
}

type StateWrapper = Arc<Mutex<MoviesState>>;

fn state_init() -> StateWrapper { 
    Arc::new(
        Mutex::new(
            MoviesState::new()
        )
    )
}

#[axum::debug_handler]
async fn post_handler(State(state): State<StateWrapper>, Json(movie): Json<Movie>) -> Result<(), StatusCode> { 
    let mut state_ref = state.lock().await;
    if state_ref.movies.contains_key(&movie.id) {
        // Handle attempts to submit a movie with the same ID as another movie already in our database.
        return Err(StatusCode::BAD_REQUEST);
    }
    debug!("Adding movie {}", movie.name);
    state_ref.movies.insert(movie.id.clone(), movie);
    debug!("Current application movie table is: {:#?}", state_ref.movies);
    Ok(())
}

#[axum::debug_handler]
async fn get_handler(Path(id): Path<String>, State(state): State<StateWrapper>, ) -> Result<String, StatusCode> { 
    let state_ref = state.lock().await;
    if let Some(movie) = state_ref.movies.get(&id) { 
        match serde_json::to_string_pretty(movie) {
            Ok(serialized) => Ok(serialized),
            Err(_e) => Err(StatusCode::NOT_FOUND),
        }
    }
    else { 
        Err(StatusCode::NOT_FOUND)
    }
}

#[tokio::main]
async fn main() {
    // Create Axum server with the following endpoints:
    // 1. GET /movie/{id} - This should return back a movie given the id
    // 2. POST /movie - this should save move in a DB (HashMap<String, Movie>). This movie will be sent
    // via a JSON payload.
    
    let state = state_init();
    
    // As a bonus: implement a caching layer so we don't need to make expensive "DB" lookups, etc.
    let state_clone = state.clone();
    let app = Router::new()
        .route("/movie", post(post_handler))
        .route("/movie/{id}",
            get({
                move |path| get_handler(path, State(state_clone))
            }),
        )
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1234").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
