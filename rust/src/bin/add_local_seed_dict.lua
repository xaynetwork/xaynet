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