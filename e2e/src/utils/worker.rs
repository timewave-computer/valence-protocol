use async_trait::async_trait;
use log::{error, info};
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fs, path::Path};

#[async_trait]
pub trait ValenceWorker {
    fn get_name(&self) -> String;

    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;

    fn start(self) -> std::thread::JoinHandle<()>
    where
        Self: Sized + Send + 'static,
    {
        info!("Starting worker: {}", self.get_name());

        // start the worker in its own thread to own the runtime
        std::thread::spawn(move || {
            // create the tokio runtime
            let rt = tokio::runtime::Runtime::new().unwrap();

            // start looping inside this runtime
            rt.block_on(async {
                let mut worker = self;
                let worker_name = worker.get_name();

                info!("{worker_name}: Worker started in new runtime");

                loop {
                    match worker.cycle().await {
                        Ok(_) => {
                            info!("{worker_name}: cycle completed successfully");
                        }
                        Err(e) => {
                            error!("{worker_name}: error in cycle: {:?}", e);
                            // sleep a little just in case
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }
                    }
                }
            });
        })
    }
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
