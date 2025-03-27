use std::sync::{Arc, RwLock};

use notify::{Error, Event, INotifyWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::Path;

pub trait CacheKey {
    fn id(&self) -> String;
}

/// The key is the chip id
// type ChipCache = HashMap<String, ChipInfo>;

pub type Cache<T> = Arc<RwLock<HashMap<String, T>>>;

fn load_cache_from_file<T: CacheKey + serde::de::DeserializeOwned>(
    path: &str,
) -> Result<HashMap<String, T>, Box<dyn std::error::Error>> {
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
) -> Result<(Cache<T>, INotifyWatcher), Box<dyn std::error::Error>> {
    let config = load_cache_from_file(path)?;

    // We wrap the data a mutex under an atomic reference counted pointer
    // to guarantee that the config won't be read and written to at the same time.
    // To learn about how that works,
    // please check out the [Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html) chapter of the Rust book.
    let config = Arc::new(RwLock::new(config));
    let cloned_config = Arc::clone(&config);

    let cloned_path = path.to_owned();

    let conf = notify::Config::default();

    // We listen to file changes by giving Notify
    // a function that will get called when events happen
    let mut watcher =
        // To make sure that the config lives as long as the function
        // we need to move the ownership of the config inside the function
        // To learn more about move please read [Using move Closures with Threads](https://doc.rust-lang.org/book/ch16-01-threads.html?highlight=move#using-move-closures-with-threads)
        RecommendedWatcher::new(move |result: Result<Event, Error>| {
            let event = result.unwrap();

            if event.kind.is_modify() {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                match load_cache_from_file(&cloned_path) {
                    Ok(new_config) => {
                        tracing::info!("Successfully reloaded cache from file: {}", &cloned_path);
                        *cloned_config.try_write().unwrap() = new_config
                    },
                    Err(error) => {
                        tracing::error!("Error reloading cache from file {}: {:?}", &cloned_path,error)
                    },
                }
            }
        },conf)?;

    watcher.watch(Path::new(path), RecursiveMode::Recursive)?;
    Ok((config, watcher))
}
