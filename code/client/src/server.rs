#[derive(Clone)]
pub(crate) struct Server;
pub(crate) enum ServerError {
    Error,
}

impl Server {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, servername: String) -> bool {
        servername == "sigma.net"
    }

    pub fn create_room(&self) -> bool {
        true
    }

    pub fn join_room(&self, room_code: u32) -> bool {
        room_code == 2137
    }

    pub fn get_player_list(&self) -> Result<Vec<String>, ServerError> {
        Ok(vec![
            "sigma1".into(),
            "xxxDestroyerxxx".into(),
            "sigma2".into(),
        ])
    }

    pub fn leave(&self) {}

    pub fn get_room_code(&self) -> Result<u32, ServerError> {
        Ok(2317)
    }

    pub fn start_game(&self) -> bool {
        true
    }
}
