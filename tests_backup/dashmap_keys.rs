use dashmap::DashMap;
use redgold::util;

#[test]
fn test_keys() {
    let map: DashMap<[u8; 32], &str> = DashMap::new();
    let str1 = "asdf";
    let str2 = "asdf2";
    let res = util::sha256(str1.as_ref());
    let res2 = util::sha256(str2.as_ref());
    map.insert(res, str1);
    map.insert(res2, str2);

    let v = map.get(&res).unwrap();
    assert_eq!(str1, *v.value());

    let v2 = map.get(&res2).unwrap();
    assert_eq!(str2, *v2.value());
}

#[test]
fn test_keys_vec() {
    let map: DashMap<&Vec<u8>, &str> = DashMap::new();
    let str1 = "asdf";
    let str2 = "asdf2";
    let res = &util::sha256(str1.as_ref()).to_vec();
    let res2 = &util::sha256(str2.as_ref()).to_vec();
    map.insert(res, str1);
    map.insert(res2, str2);

    let v = map.get(&res).unwrap();
    assert_eq!(str1, *v.value());

    let v2 = map.get(&res2).unwrap();
    assert_eq!(str2, *v2.value());
}
