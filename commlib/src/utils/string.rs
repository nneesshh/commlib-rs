//! Commlib: StringUtil

/// 字符串引用转换成目标类型 T 值
#[inline(always)]
pub fn string_to_value<T>(s: &str) -> T
where
    T: std::str::FromStr + Default,
{
    if let Ok(v) = s.parse::<T>() {
        v
    } else {
        T::default()
    }
}

/// 分割字符串，结果转换成目标类型 Vec<T>
#[inline(always)]
pub fn split_string_to_vec<T>(s: &str, sep: &str) -> Vec<T>
where
    T: std::str::FromStr + Default,
{
    s.split(sep)
        .map(|e| {
            if let Ok(x) = e.parse::<T>() {
                x
            } else {
                T::default()
            }
        })
        .collect::<Vec<T>>()
}

/// 分割字符串，结果转换成目标类型 hashbrown::HashSet<T>
#[inline(always)]
pub fn split_string_to_set<T>(s: &str, sep: &str) -> hashbrown::HashSet<T>
where
    T: std::str::FromStr + Default + std::hash::Hash + std::cmp::Eq,
{
    s.split(sep)
        .map(|e| {
            if let Ok(x) = e.parse::<T>() {
                x
            } else {
                T::default()
            }
        })
        .collect::<hashbrown::HashSet<T>>()
}

/// 分割字符串，结果转换成目标类型 Vec<Vec<T>>
#[inline(always)]
pub fn split_string_to_vec_vec<T>(s: &str, sep: &str) -> Vec<Vec<T>>
where
    T: std::str::FromStr + Default,
{
    let first_vec = split_string_to_vec::<String>(s, "|");
    let mut vec_vec = Vec::<Vec<T>>::with_capacity(first_vec.len());
    for first_it in &first_vec {
        let second_vec = split_string_to_vec::<T>(first_it, sep);
        vec_vec.push(second_vec);
    }
    vec_vec
}

/// 分割字符串，结果转换成目标类型 hashbrown::HashMap<K, V>
#[inline(always)]
pub fn split_string_to_table<K, V>(s: &str, sep: &str) -> hashbrown::HashMap<K, V>
where
    K: std::str::FromStr + Default + std::hash::Hash + std::cmp::Eq,
    V: std::str::FromStr + Default,
{
    let first_vec = split_string_to_vec::<String>(s, "|");
    let mut table = hashbrown::HashMap::<K, V>::with_capacity(first_vec.len());
    for first_it in &first_vec {
        let second_vec = split_string_to_vec::<String>(first_it, sep);
        if second_vec.len() >= 2 {
            let k = string_to_value::<K>(&second_vec[0]);
            let v = string_to_value::<V>(&second_vec[1]);
            table.insert(k, v);
        }
    }
    table
}

/// 分割字符串，结果转换成目标类型 std::tuple(U, V)
#[inline(always)]
pub fn split_string_to_pair<U, V>(s: &str, sep: &str) -> (U, V)
where
    U: std::str::FromStr + Default,
    V: std::str::FromStr + Default,
{
    let vec = split_string_to_vec::<String>(s, sep);
    if vec.len() >= 2 {
        let u = string_to_value::<U>(&vec[0]);
        let v = string_to_value::<V>(&vec[1]);
        (u, v)
    } else {
        (U::default(), V::default())
    }
}

/// 分割字符串，结果转换成目标类型 Vec<(U, V)>
#[inline(always)]
pub fn split_string_to_vec_pair<U, V>(s: &str, sep: &str) -> Vec<(U, V)>
where
    U: std::str::FromStr + Default,
    V: std::str::FromStr + Default,
{
    let first_vec = split_string_to_vec::<String>(s, "|");
    let mut vec_pair = Vec::<(U, V)>::with_capacity(first_vec.len());
    for first_it in &first_vec {
        let second_vec = split_string_to_vec::<String>(first_it, sep);
        let pair = if second_vec.len() >= 2 {
            let u = string_to_value::<U>(&second_vec[0]);
            let v = string_to_value::<V>(&second_vec[1]);
            (u, v)
        } else {
            (U::default(), V::default())
        };
        vec_pair.push(pair);
    }
    vec_pair
}

#[allow(dead_code)]
pub struct StringUtil();
