use std::sync::Arc;

use crate::RedisReply;

use super::RedisClient;

/// HSET
#[inline(always)]
pub fn hset<F>(cli: &Arc<RedisClient>, key: &str, field: &str, value: &str, cb: F)
where
    F: Fn(RedisReply) + Send + Sync + 'static,
{
    let cmd = vec![
        "HSET".to_owned(),
        key.to_owned(),
        field.to_owned(),
        value.to_owned(),
    ];
    cli.send(cmd, cb);
}

/// HSET blocking
#[inline(always)]
pub fn hset_blocking(cli: &Arc<RedisClient>, key: &str, field: &str, value: &str) -> RedisReply {
    let cmd = vec![
        "HSET".to_owned(),
        key.to_owned(),
        field.to_owned(),
        value.to_owned(),
    ];
    let prms = cli.send_and_commit_blocking(cmd);
    prms.wait()
}

/// HGET
#[inline(always)]
pub fn hget<F>(cli: &Arc<RedisClient>, key: &str, field: &str, cb: F)
where
    F: Fn(RedisReply) + Send + Sync + 'static,
{
    let cmd = vec!["HSET".to_owned(), key.to_owned(), field.to_owned()];
    cli.send(cmd, cb);
}

/// HGET blocking
#[inline(always)]
pub fn hget_blocking(cli: &Arc<RedisClient>, key: &str, field: &str) -> RedisReply {
    let cmd = vec!["HSET".to_owned(), key.to_owned(), field.to_owned()];
    let prms = cli.send_and_commit_blocking(cmd);
    prms.wait()
}

/// HGETALL
#[inline(always)]
pub fn hgetall<F>(cli: &Arc<RedisClient>, key: &str, cb: F)
where
    F: Fn(RedisReply) + Send + Sync + 'static,
{
    let cmd = vec!["HGETALL".to_owned(), key.to_owned()];
    cli.send(cmd, cb);
}

/// HGETALL blocking
#[inline(always)]
pub fn hgetall_blocking(cli: &Arc<RedisClient>, key: &str) -> RedisReply {
    let cmd = vec!["HGETALL".to_owned(), key.to_owned()];
    let prms = cli.send_and_commit_blocking(cmd);
    prms.wait()
}

/// XADD key [NOMKSTREAM] [<MAXLEN | MINID> [= | ~] threshold [LIMIT count]] <* | id> field value [field value ...]
#[inline(always)]
pub fn xadd<F>(cli: &Arc<RedisClient>, key: &str, id: &str, field_members: Vec<String>, cb: F)
where
    F: Fn(RedisReply) + Send + Sync + 'static,
{
    // field_members MUST in the (key, value, ...) pattern
    assert!(field_members.len() >= 2);

    let pre_cmd = vec!["XADD".to_owned(), key.to_owned(), id.to_owned()];

    // append field members
    let cmd = [&pre_cmd[..], &field_members[..]].concat();

    //
    cli.send(cmd, cb);
}

/// XADD blocking
#[inline(always)]
pub fn xadd_blocking(
    cli: &Arc<RedisClient>,
    key: &str,
    id: &str,
    field_members: Vec<String>,
) -> RedisReply {
    // field_members MUST in the (key, value, ...) pattern
    assert!(field_members.len() >= 2);

    let pre_cmd = vec!["XADD".to_owned(), key.to_owned(), id.to_owned()];

    // append field members
    let cmd = [&pre_cmd[..], &field_members[..]].concat();

    //
    let prms = cli.send_and_commit_blocking(cmd);
    prms.wait()
}

/// XREAD [COUNT count] [BLOCK milliseconds] STREAMS key [key ...] id [id ...]
#[inline(always)]
pub fn xread<F>(
    cli: &Arc<RedisClient>,
    count: usize,
    block: u64,
    (streams, ids): &(Vec<String>, Vec<String>),
    cb: F,
) where
    F: Fn(RedisReply) + Send + Sync + 'static,
{
    assert!(count > 0);

    let pre_cmd = vec![
        "XREAD".to_owned(),
        "COUNT".to_owned(),
        count.to_string(),
        "BLOCK".to_owned(),
        block.to_string(),
        "STREAMS".to_owned(),
    ];

    // append streams and ids
    let cmd = [&pre_cmd[..], &streams[..], &ids[..]].concat();

    //
    cli.send(cmd, cb);
}

/// XREAD blocking
#[inline(always)]
pub fn xread_blocking(
    cli: &Arc<RedisClient>,
    count: usize,
    block: u64,
    (streams, ids): &(Vec<String>, Vec<String>),
) -> RedisReply {
    assert!(count > 0);

    let pre_cmd = vec![
        "XREAD".to_owned(),
        "COUNT".to_owned(),
        count.to_string(),
        "BLOCK".to_owned(),
        block.to_string(),
        "STREAMS".to_owned(),
    ];

    // append streams and ids
    let cmd = [&pre_cmd[..], &streams[..], &ids[..]].concat();

    //
    let prms = cli.send_and_commit_blocking(cmd);
    prms.wait()
}
