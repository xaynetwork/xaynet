use redis::AsyncCommands;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> redis::RedisResult<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_async_connection().await?;

    con.hset(
        "sum_dict",
        &["sum_p1_1", "enc_key_1"],
        &["sum_p1_2", "enc_key_2"],
    )
    .await?;

    // Lua scripts allow us to perform atomic operation on redis.
    // from https://redis.io/commands/eval
    // > Redis uses the same Lua interpreter to run all the commands. Also Redis guarantees that a
    //   script is executed in an atomic way: no other script or Redis command will be executed
    //   while a script is being executed. This semantic is similar to the one of MULTI / EXEC. 
    //   From the point of view of all the other clients the effects of a script are either still 
    //   not visible or already completed.
    //   However this also means that executing slow scripts is not a good idea. It is not hard to 
    //   create fast scripts, as the script overhead is very low, but if you are going to use slow 
    //   scripts you should be aware that while the script is running no other client can execute 
    //   commands.

    // Redis provides two types of parameters: "KEYS" and "ARGV". Both types are arrays.
    // Keys are the names of all keys accessed by the script.
    // Arguments are anything excluding any names of keys.
    // The distinction between the two types is particularly important when using a Redis cluster.
    // In a Redis cluster, the dataset is automatically split among multiple nodes.
    // In order for the Redis cluster to forward the request to the corresponding cluster node
    // (which manages the key), the cluster needs to know which keys are being accessed / updated
    // by the script.
    // However, the following script does not follow the rule to make the script easier to read.
    // https://redis.io/commands/eval
    let add_local_seed_dict = redis::Script::new(
        r#"
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
    
    return 1
    "#,
    );

    let result = add_local_seed_dict
        .key(&["sum_p1_1", "seed_1", "sum_p1_2", "seed_2"])
        .arg("update_pk")
        .invoke_async(&mut con)
        .await?;
    info!("Result {:?}", result);

    // should fail unknown sum_pk: sum_p1_3
    add_local_seed_dict
        .key(&["sum_p1_3", "seed_1", "sum_p1_2", "seed_2"])
        .arg("update_pk")
        .invoke_async(&mut con)
        .await?;
    Ok(())
}
