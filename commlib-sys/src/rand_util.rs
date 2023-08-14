//! Commlib: RandUtil

use std::collections::LinkedList;
//use rand::distributions::uniform::SampleUniform;
use rand::rngs::{Mt64, SmallRng};
use rand::{seq::SliceRandom, Rng, SeedableRng};
use std::time::SystemTime;

/// 创建生成器 Mt64
pub fn create_mt64(seed: u64) -> Mt64 {
    Mt64::seed_from_u64(seed)
}

/// 创建生成器 SmallRng
pub fn create_small_rng(seed: u64) -> SmallRng {
    SmallRng::seed_from_u64(seed)
}

thread_local! {
    /// 全局生成器
    static G_SMALL_RNG: std::cell::UnsafeCell<SmallRng> = {
        let now_in_secs = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        std::cell::UnsafeCell::new(create_small_rng(now_in_secs))
    };
}

///
pub fn gen_password(password_len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789)(*&^%$#@!~";

    G_SMALL_RNG.with(|rng| {
        let rng_mut = unsafe { &mut (*rng.get()) };

        let password: String = (0..password_len)
            .map(|_| {
                let idx = rand_range(rng_mut, 0, CHARSET.len() as i32);
                CHARSET[idx as usize] as char
            })
            .collect();
        password
    })
}

/*let rng = rand::thread_rng();
let x = rng.sample(rand::distributions::Uniform::new(10u32, 15));
// Type annotation requires two types, the type and distribution; the
// distribution can be inferred.
let y = rng.sample::<u16, _>(rand::distributions::Uniform::new(10, 15));*/

///
#[inline(always)]
pub fn rand_between(start: i32, end: i32) -> i32 {
    G_SMALL_RNG.with(|rng| {
        let rng_mut = unsafe { &mut (*rng.get()) };
        rand_between2(start, end, rng_mut)
    })
}

///
#[inline(always)]
pub fn rand_between2(start: i32, end: i32, rng: &mut SmallRng) -> i32 {
    rand_range(rng, start, end)
}

///
pub fn rand_between_exclusive(start: i32, end: i32, exclude_list: &LinkedList<i32>) -> i32 {
    let max_count = std::cmp::max(100, exclude_list.len());
    let mut count = 0;
    while count < max_count {
        let n = rand_between(start, end);
        let mut found = false;
        for it in exclude_list {
            if *it == n {
                found = true;
            }
        }
        if !found {
            return n;
        } else {
            count = count + 1;
        }
    }
    start
}

///
pub fn rand_between_exclusive_i8(start: i8, end: i8, exclude_list: &LinkedList<i8>) -> i8 {
    let max_count = std::cmp::max(100, exclude_list.len());
    let mut count = 0;
    while count < max_count {
        let n = rand_between(start as i32, end as i32);
        let mut found = false;
        for it in exclude_list {
            if *it as i32 == n {
                found = true;
            }
        }
        if !found {
            return n as i8;
        } else {
            count = count + 1;
        }
    }
    start
}

///
pub fn rand_many(start: i32, end: i32, count: usize) -> Vec<i32> {
    let mut vec = Vec::<i32>::with_capacity(count);
    while vec.len() < count {
        let n = rand_between(start, end);
        vec.push(n);
    }
    vec
}

///
pub fn rand_one_from_hashmap<T>(table: &hashbrown::HashMap<u32, T>) -> &T {
    let mut sum = 0_u32;
    for it in table {
        sum += it.0;
    }

    let n = rand_between(0, sum as i32) as u32;
    let mut cur = 0_u32;
    for it2 in table {
        cur += it2.0;
        if cur >= n {
            return it2.1;
        }
    }
    std::unreachable!()
}

///
#[inline(always)]
pub fn rand_ratio_(numerator: u32, denominator: u32) -> bool {
    G_SMALL_RNG.with(|rng| {
        let rng_mut = unsafe { &mut (*rng.get()) };
        rand_ratio(rng_mut, numerator, denominator)
    })
}

///
#[inline(always)]
pub fn rand_shuffle(vec: &mut Vec<i32>) {
    G_SMALL_RNG.with(|rng| {
        let rng_mut = unsafe { &mut (*rng.get()) };
        vec.shuffle(rng_mut);
    });
}

#[inline(always)]
fn rand_range<R>(rng: &mut R, start: i32, end: i32) -> i32
where
    R: Rng,
{
    rng.gen_range(start..=end)
}

#[inline(always)]
fn rand_ratio<R>(rng: &mut R, numerator: u32, denominator: u32) -> bool
where
    R: Rng,
{
    rng.gen_ratio(numerator, denominator)
}

#[allow(dead_code)]
pub struct RandUtil();
