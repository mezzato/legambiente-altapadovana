use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use notify::{Error, Event, INotifyWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;

use futures::{
    SinkExt, StreamExt,
    channel::mpsc::{Receiver, channel},
};

use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(10);

    let conf = notify::Config::default();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let watcher = RecommendedWatcher::new(
        move |result: std::result::Result<Event, Error>| {
            futures::executor::block_on(async {
                match &result {
                    Ok(r) => tracing::info!("Event detected: {:?}", r.kind),
                    Err(e) => tracing::info!("Event error detected: {}", e),
                }

                tx.send(result).await.unwrap();
            })
        },
        conf,
    )?;

    Ok((watcher, rx))
}

fn async_debounce_watcher() -> notify::Result<(
    Debouncer<RecommendedWatcher, RecommendedCache>,
    Receiver<DebounceEventResult>,
)> {
    let (mut tx, rx) = channel(1);

    // Select recommended watcher for debouncer.
    // Using a callback here, could also be a channel.
    let debouncer = new_debouncer(
        Duration::from_secs(2),
        None,
        move |result: DebounceEventResult| {
            tracing::info!("Event detected",);
            futures::executor::block_on(async {
                tx.send(result).await.unwrap();
            })
        },
    )?;

    Ok((debouncer, rx))
}

pub trait CacheKey {
    fn id(&self) -> String;
}

/// The key is the chip id
// type ChipCache = HashMap<String, ChipInfo>;

pub type Cache<T> = Arc<RwLock<HashMap<String, T>>>;

fn load_cache_from_file<T: CacheKey + serde::de::DeserializeOwned>(
    path: &str,
) -> std::result::Result<HashMap<String, T>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let file_size = file.metadata()?.len();

    if file_size == 0 {
        return Err("the config file is empty.".into());
    }

    let reader = std::io::BufReader::new(file);

    let mut cache = HashMap::new();

    let mut rdr = csv::Reader::from_reader(reader);
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let record: T = result?;
        cache.insert(record.id().clone(), record);
    }

    Ok(cache)
}

pub fn load_cache<T: CacheKey + serde::de::DeserializeOwned + Send + Sync + 'static>(
    path: &str,
) -> std::result::Result<(Cache<T>, INotifyWatcher), Box<dyn std::error::Error>> {
    let config = load_cache_from_file(path)?;

    // We wrap the data a mutex under an atomic reference counted pointer
    // to guarantee that the config won't be read and written to at the same time.
    // To learn about how that works,
    // please check out the [Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html) chapter of the Rust book.
    let config = Arc::new(RwLock::new(config));
    let cloned_config = Arc::clone(&config);

    let cloned_path = path.to_owned();

    let (mut watcher, mut rx) = async_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    tokio::spawn(async move {
        while let Some(res) = rx.next().await {
            match res {
                Ok(event) => {
                    //for event in events {
                    if event.kind.is_modify() {
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                        match load_cache_from_file(&cloned_path) {
                            Ok(new_config) => {
                                tracing::info!(
                                    "Successfully reloaded cache from file: {}",
                                    &cloned_path
                                );
                                *cloned_config.try_write().unwrap() = new_config
                            }
                            Err(error) => {
                                tracing::error!(
                                    "Error reloading cache from file {}: {:?}",
                                    &cloned_path,
                                    error
                                )
                            }
                        }
                    }
                    // }
                }
                Err(e) => tracing::error!("Error watching file {}: {:?}", &cloned_path, e),
                /*
                Err(errors) => errors
                    .iter()
                    .for_each(|e| tracing::error!("Error watching file {}: {:?}", &cloned_path, e)),
                    */
            }
        }
    });

    Ok((config, watcher))
}
