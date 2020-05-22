

-- update phase
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


-- idle phase
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

