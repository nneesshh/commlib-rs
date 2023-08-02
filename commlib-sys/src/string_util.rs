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
    let vec1 = split_string_to_vec::<String>(s, "|");
    let mut vec_vec = Vec::<Vec<T>>::with_capacity(vec1.len());
    for it1 in &vec1 {
        let tvec = split_string_to_vec::<T>(it1, sep);
        vec_vec.push(tvec);
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
    let vec1 = split_string_to_vec::<String>(s, "|");
    let mut table = hashbrown::HashMap::<K, V>::with_capacity(vec1.len());
    for it1 in &vec1 {
        let kvvec = split_string_to_vec::<String>(it1, sep);
        if kvvec.len() >= 2 {
            let k = string_to_value::<K>(&kvvec[0]);
            let v = string_to_value::<V>(&kvvec[1]);
            table.insert(k, v);
        }
    }
    table
}

#[allow(dead_code)]
pub struct StringUtil();
