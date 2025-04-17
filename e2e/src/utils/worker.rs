use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fs, path::Path};

#[async_trait]
pub trait ValenceWorker: ValenceWorkerTomlSerde {
    fn get_name(&self) -> String;
}

pub trait ValenceWorkerTomlSerde: Sized + Serialize + DeserializeOwned {
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let toml_string = toml::to_string(self)?;
        fs::write(path, toml_string)?;
        Ok(())
    }
}
