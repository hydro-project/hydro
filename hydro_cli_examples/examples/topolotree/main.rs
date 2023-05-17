use hydroflow::serde::{Deserialize, Serialize};
use hydroflow::util::cli::{ConnectedBidi, ConnectedSink, ConnectedSource};
use hydroflow::util::{deserialize_from_bytes, serialize_to_bytes};
use hydroflow::{hydroflow_syntax, tokio};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct IncrementRequest {
    tweet_id: u64,
    likes: i32,
}

#[derive(Serialize, Deserialize, Default, Copy, Clone, Debug)]
struct TimestampedValue<T> {
    pub value: T,
    pub timestamp: u32,
}

impl<T> TimestampedValue<T> {
    pub fn merge_from(&mut self, other: TimestampedValue<T>) -> bool {
        if other.timestamp > self.timestamp {
            self.value = other.value;
            self.timestamp = other.timestamp;
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, updater: impl Fn(&T) -> T) {
        self.value = updater(&self.value);
        self.timestamp += 1;
    }
}

impl<T> PartialEq for TimestampedValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl<T> Eq for TimestampedValue<T> {}

impl<T> Hash for TimestampedValue<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.timestamp.hash(state);
    }
}

#[tokio::main]
async fn main() {
    let mut ports = hydroflow::util::cli::init().await;
    let increment_requests = ports
        .port("increment_requests")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let query_responses = ports
        .port("query_responses")
        .connect::<ConnectedBidi>()
        .await
        .into_sink();

    let to_parent = ports
        .port("to_parent")
        .connect::<ConnectedBidi>()
        .await
        .into_sink();

    let from_parent = ports
        .port("from_parent")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let to_left = ports
        .port("to_left")
        .connect::<ConnectedBidi>()
        .await
        .into_sink();

    let from_left = ports
        .port("from_left")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let to_right = ports
        .port("to_right")
        .connect::<ConnectedBidi>()
        .await
        .into_sink();

    let from_right = ports
        .port("from_right")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let f1 = async move {
        #[cfg(target_os = "linux")]
        loop {
            let x = procinfo::pid::stat_self().unwrap();
            let bytes = x.rss * 1024 * 4;
            println!("memory,{}", bytes);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    };

    type UpdateType = (u64, i32);

    let df = hydroflow_syntax! {
        from_parent = source_stream(from_parent)
            -> map(|x| deserialize_from_bytes::<Vec<(u64, TimestampedValue<i32>)>>(x.unwrap()).unwrap())
            -> fold::<'static>(
                (HashMap::<u64, TimestampedValue<i32>>::new(), HashSet::new(), 0),
                |(mut prev, mut modified_tweets, prev_tick), req: Vec<(u64, TimestampedValue<i32>)>| {
                    if prev_tick != context.current_tick() {
                        modified_tweets.clear();
                    }

                    for (k, v) in req {
                        let updated = if let Some(e) = prev.get_mut(&k) {
                            e.merge_from(v)
                        } else {
                            prev.insert(k, v);
                            true
                        };

                        if updated {
                            modified_tweets.insert(k);
                        }
                    }

                    (prev, modified_tweets, context.current_tick())
                }
            )
            -> filter(|(_, _, tick)| *tick == context.current_tick())
            -> flat_map(|(state, modified_tweets, _)| modified_tweets.iter().map(|t| (*t, *state.get(t).unwrap())).collect::<Vec<_>>())
            -> tee();

        from_left = source_stream(from_left)
            -> map(|x| deserialize_from_bytes::<Vec<(u64, TimestampedValue<i32>)>>(x.unwrap()).unwrap())
            -> fold::<'static>(
                (HashMap::<u64, TimestampedValue<i32>>::new(), HashSet::new(), 0),
                |(mut prev, mut modified_tweets, prev_tick), req: Vec<(u64, TimestampedValue<i32>)>| {
                    if prev_tick != context.current_tick() {
                        modified_tweets.clear();
                    }

                    for (k, v) in req {
                        let updated = if let Some(e) = prev.get_mut(&k) {
                            e.merge_from(v)
                        } else {
                            prev.insert(k, v);
                            true
                        };

                        if updated {
                            modified_tweets.insert(k);
                        }
                    }

                    (prev, modified_tweets, context.current_tick())
                }
            )
            -> filter(|(_, _, tick)| *tick == context.current_tick())
            -> flat_map(|(state, modified_tweets, _)| modified_tweets.iter().map(|t| (*t, *state.get(t).unwrap())).collect::<Vec<_>>())
            -> tee();

        from_right = source_stream(from_right)
            -> map(|x| deserialize_from_bytes::<Vec<(u64, TimestampedValue<i32>)>>(x.unwrap()).unwrap())
            -> fold::<'static>(
                (HashMap::<u64, TimestampedValue<i32>>::new(), HashSet::new(), 0),
                |(mut prev, mut modified_tweets, prev_tick), req: Vec<(u64, TimestampedValue<i32>)>| {
                    if prev_tick != context.current_tick() {
                        modified_tweets.clear();
                    }

                    for (k, v) in req {
                        let updated = if let Some(e) = prev.get_mut(&k) {
                            e.merge_from(v)
                        } else {
                            prev.insert(k, v);
                            true
                        };

                        if updated {
                            modified_tweets.insert(k);
                        }
                    }

                    (prev, modified_tweets, context.current_tick())
                }
            )
            -> filter(|(_, _, tick)| *tick == context.current_tick())
            -> flat_map(|(state, modified_tweets, _)| modified_tweets.iter().map(|t| (*t, *state.get(t).unwrap())).collect::<Vec<_>>())
            -> tee();

        from_local = source_stream(increment_requests)
            -> map(|x| deserialize_from_bytes::<IncrementRequest>(&x.unwrap()).unwrap())
            -> map(|x| (x.tweet_id, x.likes))
            -> fold::<'static>(
                (HashMap::<u64, TimestampedValue<i32>>::new(), HashSet::new(), 0),
                |(mut prev, mut modified_tweets, prev_tick), req: UpdateType| {
                    if prev_tick != context.current_tick() {
                        modified_tweets.clear();
                    }

                    prev.entry(req.0).or_default().update(|v| v + req.1);
                    modified_tweets.insert(req.0);
                    (prev, modified_tweets, context.current_tick())
                }
            )
            -> filter(|(_, _, tick)| *tick == context.current_tick())
            -> flat_map(|(state, modified_tweets, _)| modified_tweets.iter().map(|t| (*t, *state.get(t).unwrap())).collect::<Vec<_>>())
            -> tee();

        to_right = merge();

        from_parent -> map(|v| (0, v)) -> to_right;
        from_left -> map(|v| (1, v)) -> to_right;
        from_local -> map(|v| (2, v)) -> to_right;

        to_right
            -> fold::<'static>(
                (vec![HashMap::<u64, TimestampedValue<i32>>::new(); 3], HashMap::<u64, TimestampedValue<i32>>::new(), HashSet::new(), 0),
                |(mut each_source, mut acc_source, mut modified_tweets, prev_tick), (source_i, (key, v))| {
                    if prev_tick != context.current_tick() {
                        modified_tweets.clear();
                    }

                    let updated = each_source[source_i].entry(key).or_default().merge_from(v);

                    if updated {
                        acc_source.entry(key).or_default().update(|_| each_source.iter().map(|s| s.get(&key).map(|t| t.value).unwrap_or_default()).sum());
                        modified_tweets.insert(key);
                    }

                    (each_source, acc_source, modified_tweets, context.current_tick())
                }
            )
            -> filter(|(_, _, _, tick)| *tick == context.current_tick())
            -> map(|(_, state, modified_tweets, _)| modified_tweets.iter().map(|t| (*t, *state.get(t).unwrap())).collect())
            -> map(serialize_to_bytes::<Vec<(u64, TimestampedValue<i32>)>>)
            -> dest_sink(to_right);

        to_left = merge();

        from_parent -> map(|v| (0, v)) -> to_left;
        from_right -> map(|v| (1, v)) -> to_left;
        from_local -> map(|v| (2, v)) -> to_left;

        to_left
            -> fold::<'static>(
                (vec![HashMap::<u64, TimestampedValue<i32>>::new(); 3], HashMap::<u64, TimestampedValue<i32>>::new(), HashSet::new(), 0),
                |(mut each_source, mut acc_source, mut modified_tweets, prev_tick), (source_i, (key, v))| {
                    if prev_tick != context.current_tick() {
                        modified_tweets.clear();
                    }

                    let updated = each_source[source_i].entry(key).or_default().merge_from(v);

                    if updated {
                        acc_source.entry(key).or_default().update(|_| each_source.iter().map(|s| s.get(&key).map(|t| t.value).unwrap_or_default()).sum());
                        modified_tweets.insert(key);
                    }

                    (each_source, acc_source, modified_tweets, context.current_tick())
                }
            )
            -> filter(|(_, _, _, tick)| *tick == context.current_tick())
            -> map(|(_, state, modified_tweets, _)| modified_tweets.iter().map(|t| (*t, *state.get(t).unwrap())).collect())
            -> map(serialize_to_bytes::<Vec<(u64, TimestampedValue<i32>)>>)
            -> dest_sink(to_left);

        to_parent = merge();

        from_right -> map(|v| (0, v)) -> to_parent;
        from_left -> map(|v| (1, v)) -> to_parent;
        from_local -> map(|v| (2, v)) -> to_parent;

        to_parent
            -> fold::<'static>(
                (vec![HashMap::<u64, TimestampedValue<i32>>::new(); 3], HashMap::<u64, TimestampedValue<i32>>::new(), HashSet::new(), 0),
                |(mut each_source, mut acc_source, mut modified_tweets, prev_tick), (source_i, (key, v))| {
                    if prev_tick != context.current_tick() {
                        modified_tweets.clear();
                    }

                    let updated = each_source[source_i].entry(key).or_default().merge_from(v);

                    if updated {
                        acc_source.entry(key).or_default().update(|_| each_source.iter().map(|s| s.get(&key).map(|t| t.value).unwrap_or_default()).sum());
                        modified_tweets.insert(key);
                    }

                    (each_source, acc_source, modified_tweets, context.current_tick())
                }
            )
            -> filter(|(_, _, _, tick)| *tick == context.current_tick())
            -> map(|(_, state, modified_tweets, _)| modified_tweets.iter().map(|t| (*t, *state.get(t).unwrap())).collect())
            -> map(serialize_to_bytes::<Vec<(u64, TimestampedValue<i32>)>>)
            -> dest_sink(to_parent);

        to_query = merge();

        from_parent -> map(|v| (0, v)) -> to_query;
        from_left -> map(|v| (1, v)) -> to_query;
        from_right -> map(|v| (2, v)) -> to_query;
        from_local -> map(|v| (3, v)) -> to_query;

        to_query
            -> fold::<'static>(
                (vec![HashMap::<u64, TimestampedValue<i32>>::new(); 4], HashMap::<u64, TimestampedValue<i32>>::new(), HashSet::new(), 0),
                |(mut each_source, mut acc_source, mut modified_tweets, prev_tick), (source_i, (key, v))| {
                    if prev_tick != context.current_tick() {
                        modified_tweets.clear();
                    }

                    let updated = each_source[source_i].entry(key).or_default().merge_from(v);

                    if updated {
                        acc_source.entry(key).or_default().update(|_| each_source.iter().map(|s| s.get(&key).map(|t| t.value).unwrap_or_default()).sum());
                        modified_tweets.insert(key);
                    }

                    (each_source, acc_source, modified_tweets, context.current_tick())
                }
            )
            -> filter(|(_, _, _, tick)| *tick == context.current_tick())
            -> flat_map(|(_, state, modified_tweets, _)| modified_tweets.iter().map(|t| (*t, state.get(t).unwrap().value)).collect::<Vec<_>>())
            -> map(serialize_to_bytes::<(u64, i32)>)
            -> dest_sink(query_responses);
    };

    // initial memory
    #[cfg(target_os = "linux")]
    {
        let x = procinfo::pid::stat_self().unwrap();
        let bytes = x.rss * 1024 * 4;
        println!("memory,{}", bytes);
    }

    let f1_handle = tokio::spawn(f1);
    hydroflow::util::cli::launch_flow(df).await;
    f1_handle.abort();
}
