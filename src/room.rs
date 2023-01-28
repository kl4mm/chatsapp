use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

use redis::{AsyncCommands, Client};

pub enum RoomEvent<'a> {
    Chat(&'a str),
    Join,
    Leave,
}

#[derive(Debug)]
pub enum RoomError {
    FailedToConnect,
    FailedToSend,
    FailedToFetch,
    FailedToCheckRoomExists,
    RoomNameTaken,
}

impl std::fmt::Display for RoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoomError::FailedToConnect => write!(f, "Error: Failed to connect\n"),
            RoomError::FailedToSend => write!(f, "Error: Failed to send\n"),
            RoomError::FailedToFetch => write!(f, "Error: Failed to fetch\n"),
            RoomError::FailedToCheckRoomExists => {
                write!(f, "Error: Failed to check if room exists\n")
            }
            RoomError::RoomNameTaken => write!(f, "Error: Room name taken\n"),
        }
    }
}

impl std::error::Error for RoomError {}

pub async fn new(redis: &Client, room: &str) -> Result<(), RoomError> {
    let mut conn = redis.get_async_connection().await.map_err(|e| {
        dbg!("{}", e);
        RoomError::FailedToConnect
    })?;

    let key = gen_key(room);

    let exists: u8 = conn.exists(&key).await.map_err(|e| {
        dbg!("{}", e);
        RoomError::FailedToCheckRoomExists
    })?;

    if exists == 1 {
        Err(RoomError::RoomNameTaken)?;
    }

    // Key, member, score
    conn.zadd(key, "Start of chat", 0).await.map_err(|e| {
        dbg!("{}", e);
        RoomError::FailedToSend
    })?;

    Ok(())
}

pub async fn list<'a>(redis: &Client) -> Result<HashSet<String>, RoomError> {
    let mut conn = redis.get_async_connection().await.map_err(|e| {
        dbg!("{}", e);
        RoomError::FailedToConnect
    })?;

    let rooms: HashSet<String> = conn.keys("room*").await.map_err(|e| {
        dbg!("{}", e);
        RoomError::FailedToFetch
    })?;

    Ok(rooms)
}

pub async fn event<'a>(
    redis: &Client,
    event: RoomEvent<'a>,
    room: &str,
    username: &str,
) -> Result<(), RoomError> {
    let mut conn = redis.get_async_connection().await.map_err(|e| {
        dbg!("{}", e);
        RoomError::FailedToConnect
    })?;

    let key = gen_key(room);
    let score = get_time_in_ms();

    match event {
        RoomEvent::Chat(message) => {
            let chat = gen_chat(username, message);

            conn.zadd(key, chat, score).await.map_err(|e| {
                dbg!("{}", e);
                RoomError::FailedToSend
            })?;
        }
        RoomEvent::Join => {
            let join = gen_join_msg(username);

            conn.zadd(key, join, score).await.map_err(|e| {
                dbg!("{}", e);
                RoomError::FailedToSend
            })?;
        }
        RoomEvent::Leave => {
            let leave = gen_leave_msg(username);

            conn.zadd(key, leave, score).await.map_err(|e| {
                dbg!("{}", e);
                RoomError::FailedToSend
            })?;
        }
    };

    Ok(())
}

fn gen_key(name: &str) -> String {
    format!("room:{}", name)
}

fn gen_chat(username: &str, message: &str) -> String {
    format!("{}: {}", username, message)
}

fn gen_join_msg(username: &str) -> String {
    format!("{} has joined the room", username)
}

fn gen_leave_msg(username: &str) -> String {
    format!("{} has left the room", username)
}

fn get_time_in_ms() -> isize {
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH).unwrap();

    since_epoch.as_millis() as isize
}
