use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use notify::{Error, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;

use futures::{
    SinkExt, StreamExt,
    channel::mpsc::{Receiver, channel},
};

use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

fn _async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(10);

    let conf = notify::Config::default();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let watcher = RecommendedWatcher::new(
        move |result: std::result::Result<Event, Error>| {
            futures::executor::block_on(async {
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
            // tracing::info!("Event detected",);
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
) -> std::result::Result<
    (
        Cache<T>,
        Arc<RwLock<Debouncer<RecommendedWatcher, RecommendedCache>>>,
    ),
    Box<dyn std::error::Error>,
> {
    let config = load_cache_from_file(path)?;

    // We wrap the data a mutex under an atomic reference counted pointer
    // to guarantee that the config won't be read and written to at the same time.
    // To learn about how that works,
    // please check out the [Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html) chapter of the Rust book.
    let config = Arc::new(RwLock::new(config));
    let cloned_config = Arc::clone(&config);

    let cloned_path = path.to_owned();

    let (mut watcher, mut rx) = async_debounce_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path, RecursiveMode::Recursive)?;

    let watcher_lock = Arc::new(RwLock::new(watcher));
    let watcher_lock_clone = watcher_lock.clone();

    tokio::spawn(async move {
        loop {
            'outer: while let Some(res) = rx.next().await {
                match res {
                    Ok(events) => {
                        for event in events {
                            if event.kind.is_modify() {
                                // std::thread::sleep(std::time::Duration::from_millis(1000));
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
                            } else if event.kind.is_remove() {
                                // reset the watcher
                                tracing::info!(
                                    "Trying to reset watcher for file: {}",
                                    &cloned_path
                                );
                                break 'outer;
                            }
                        }
                    }
                    Err(errors) => errors.iter().for_each(|e| {
                        tracing::error!("Error watching file {}: {:?}", &cloned_path, e)
                    }),
                }
            }

            let mut watcher = match async_debounce_watcher() {
                Ok((watcher, r)) => {
                    rx = r;
                    watcher
                }
                Err(e) => {
                    tracing::error!("Error watching file {}: {:?}", &cloned_path, e);
                    break;
                }
            };

            // Add a path to be watched. All files and directories at that path and
            // below will be monitored for changes.
            match watcher.watch(&cloned_path, RecursiveMode::Recursive) {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Error watching file {}: {:?}", &cloned_path, e);
                    break;
                }
            }
            *watcher_lock.write().unwrap() = watcher;
            tracing::info!("Successfully reset watcher for file: {}", &cloned_path);
        }
    });

    Ok((config, watcher_lock_clone))
}
