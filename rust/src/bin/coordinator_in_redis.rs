use rand::{distributions::Alphanumeric, thread_rng, Rng};
use redis::AsyncCommands;
use tokio::{
    stream::StreamExt,
    time::{self, Duration},
};
use tracing::{info, Level};
use tracing_subscriber;
use tracing::debug;

#[tokio::main]
async fn main() -> redis::RedisResult<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_async_connection().await?;

    // init coordinator
    // phase:
    // 0 = init
    // 1 = sum
    // 2 = update
    // 3 = sum2
    con.set_multiple(&[
        ("min_sum", 1000),
        ("cur_sum", 0),
        ("min_update", 500),
        ("cur_update", 0),
        ("min_sum2", 1000),
        ("cur_sum2", 0),
        ("phase", 1),
    ])
    .await?;

    let sum_phase = redis::Script::new(
        r#"
        -- sum phase
        local phase = redis.call("GET", "phase")

        if tonumber(phase) ~= 1 then
            return redis.error_reply("Not in sum phase! Current phase: " .. phase)
        end
        
        local sum_entry_exist = redis.call("HSETNX", "sum_dict", KEYS[1], ARGV[1])
        if sum_entry_exist == 0 then
            return redis.error_reply("Pk: " .. KEYS[1] .. " already exists!")
        end
        
        local current_sum = redis.call("INCR", "cur_sum")
        
        if tonumber(current_sum) == tonumber(redis.call("GET", "min_sum")) then
            redis.call("SET", "phase", 2)
        end
        
        return 1
    "#,
    );

    let update_phase = redis::Script::new(
        r#"
        local phase = redis.call("GET", "phase")

        if tonumber(phase) ~= 2 then
            return redis.error_reply("Not in update phase! Current phase: " .. phase)
        end
        
        local update_pk = ARGV[1]
        
        -- check if the local seed dict has the same length as the sum_dict
        local seed_dict_len = table.getn(KEYS) / 2 -- KEYS is a list of key value/sum_pk seed
        local sum_dict_len = redis.call("HLEN", "sum_dict")
        if seed_dict_len ~= sum_dict_len then
            return redis.error_reply("Precondition 1 failed! seed_dict_len: " .. seed_dict_len .. " sum_dict_len: " .. sum_dict_len)
        end
        
        -- check if all pks of the local seed dict exists in sum_dict
        for i = 1, #KEYS, 2 do
            local exist_in_sum_dict = redis.call("HEXISTS", "sum_dict", KEYS[i])
            if exist_in_sum_dict == 0 then
                return redis.error_reply("Precondition 2 failed! sum_pk: " ..  KEYS[i] .. " does not exist in sum_dict" )
            end
        end
        
        -- check if one pk of the local seed dict already exists in seed_dict
        -- SADD returns 0 if the key already exists
        local exist_in_seed_dict = redis.call("SADD", "update_participants", update_pk)
        if exist_in_seed_dict == 0 then
            return redis.error_reply("Precondition 3 failed! update_pk already exists in seed_dict")
        end
        
        -- Update the seed dict
        for i = 1, #KEYS, 2 do
            redis.call("HSETNX", KEYS[i], update_pk, KEYS[i + 1])
        end
        
        local current_update = redis.call("INCR", "cur_update")
        
        if tonumber(current_update) == tonumber(redis.call("GET", "min_update")) then
            redis.call("SET", "phase", 3)
        end
        
        return 1
    "#,
    );

    let sum2_phase = redis::Script::new(
        r#"
        -- sum2 phase
        local phase = redis.call("GET", "phase")

        if tonumber(phase) ~= 3 then
            return redis.error_reply("Not in sum2 phase! Current phase: " .. phase)
        end
        
        local sum_pk_exist = redis.call("HDEL", "sum_dict", KEYS[1])
        
        if sum_pk_exist == 0 then
            return redis.error_reply("Pk: " .. KEYS[1] .. " does not exist!")
        end
        
        redis.call("ZINCRBY", "mask_dict", 1, KEYS[2])
        
        local current_sum2 = redis.call("INCR", "cur_sum2")
        
        if tonumber(current_sum2) == tonumber(redis.call("GET", "min_sum2")) then
            redis.call("SET", "phase", 0)
        end
        
        return 1
    "#,
    );

    let idle_phase = redis::Script::new(
        r#"
        local phase = redis.call("GET", "phase")

        if tonumber(phase) ~= 0 then
            return redis.error_reply("Not in idle phase! Current phase: " .. phase)
        end
        
        redis.call("INCR", "round")
        
        local sum_keys = redis.call("HKEYS", "sum_dict")
        
        redis.call("DEL", "sum_dict")
        redis.call("DEL", "update_participants")
        
        for i, sum_pk in pairs(sum_keys) do
            redis.call("DEL", sum_pk)
        end
        
        redis.call("DEL", "mask_dict")
        
        redis.call("MSET", "cur_sum", 0, "cur_update", 0, "cur_sum2", 0, "phase", 1)
        
        return 1
    "#,
    );

    let mut inval_chan = client.get_async_connection().await?;
    let id: u32 = redis::cmd("CLIENT")
        .arg("ID")
        .query_async(&mut inval_chan)
        .await?;
    let mut inval_chan = inval_chan.into_pubsub();
    inval_chan.subscribe("__redis__:invalidate").await?;

    let mut data_chan = client.get_async_connection().await?;
    redis::cmd("CLIENT")
        .arg(&["TRACKING", "on", "REDIRECT", &id.to_string()[..]])
        .query_async(&mut data_chan)
        .await?;
    // get the value of "foo"
    let mut current_phase: u32 = redis::cmd("GET")
        .arg("phase")
        .query_async(&mut data_chan)
        .await?;

    info!("Start coordinator");
    let mut inval_stream = inval_chan.on_message();
    let mut sum_pks: Vec<String> = vec![];
    let mut delay = time::delay_for(Duration::from_millis(5));

    loop {
        match current_phase {
            0 => {
                let _: redis::RedisResult<u32> = idle_phase.prepare_invoke().invoke_async(&mut con).await;
                let round: u32 = con.get("round").await.unwrap();
                info!("New round {:?}!", round);
            }
            1 => {
                debug!("Sum phase!");
                let (sum_pk, sum_encr) = gen_sum_pk();

                let result: redis::RedisResult<u32> = sum_phase
                    .key(sum_pk.clone())
                    .arg(sum_encr)
                    .invoke_async(&mut con)
                    .await;
                    debug!("Result {:?}", &result);
                if result.is_ok() {
                    sum_pks.push(sum_pk);
                }
            }
            2 => {
                debug!("Update phase!");
                let local_seed_dict = gen_local_seed_dict(&sum_pks);

                let result: redis::RedisResult<u32> = update_phase
                    .key(local_seed_dict)
                    .arg(gen_update_pk())
                    .invoke_async(&mut con)
                    .await;
                    debug!("Result {:?}", result);
            }
            3 => {
                debug!("Sum2 phase!");
                let result: redis::RedisResult<u32> = sum2_phase
                    .key(&[&sum_pks.pop().unwrap_or(String::from("xxx")), "123"])
                    .invoke_async(&mut con)
                    .await;
                    debug!("Result {:?}", result);
            }
            _ => unreachable!(),
        }

        tokio::select! {
                _ = &mut delay => {
                    ()
                }
                notification = inval_stream.next() => {
                    if  let Some(notification) = notification {
                      // the response is a bulk type: bulk(string-type)
                      let key = &notification.get_payload::<Vec<String>>().unwrap()[0];
                      info!("Invalidation notification for {:?}", key);
                      let updated_value: u32 = redis::cmd("GET")
                          .arg(key)
                          .query_async(&mut data_chan)
                          .await?;
                      current_phase = updated_value;
                      info!("Update value of {:?} is: {:?}", key, updated_value);
                    }
                }
        };
    }
}

fn gen_sum_pk() -> (String, String) {
    (
        thread_rng().sample_iter(&Alphanumeric).take(10).collect(),
        thread_rng().sample_iter(&Alphanumeric).take(10).collect(),
    )
}

fn gen_local_seed_dict(sum_pks: &Vec<String>) -> Vec<String> {
    let mut local_seed_dict = vec![];
    for sum_pk in sum_pks.iter() {
        local_seed_dict.push(sum_pk.clone());
        local_seed_dict.push(thread_rng().sample_iter(&Alphanumeric).take(10).collect());
    }
    local_seed_dict
}

fn gen_update_pk() -> String {
    thread_rng().sample_iter(&Alphanumeric).take(10).collect()
}
