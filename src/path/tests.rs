// use std::path::MAIN_SEPARATOR;
pub const MAIN_SEPARATOR: char = ':';
use std::io;

pub fn get_path<'a>(s: &'a str, prefix: &'a str) -> Option<&'a str> {
    // Empty strings are invalid
    if s.is_empty() {
        return None;
    }
    // The "" prefix indicates the root
    if prefix.is_empty() {
        let mut s_iter = s.split(':');
        let base = s_iter.next();
        let remainder = s_iter.next();
        if remainder.is_some() {
            return None;
        }
        return base;
    }
    let without_prefix = s.strip_prefix(prefix)?.strip_prefix(':')?;
    let mut path_split = without_prefix.split(':');
    let parent = path_split.next();
    if path_split.next().is_some() {
        None
    } else {
        parent
    }
}

#[test]
fn test_split_vectors() {
    let vectors = [
        "",
        "one",
        "one:two",
        "two",
        "two:three",
        "two:one",
        "one:four",
        "one:🥸",
        "one:🥸:⛪",
        "one:four:five",
        "one:four:two",
        "one:four:three",
        "onery",
        "oneful",
    ];
    let key = "one:four";
    for v in vectors.iter().filter(|x| get_path(x, key).is_some()) {
        // print!("Testing \"{}\": ", v);
        // if let Some(v) = get_path(v, key) {
        if let Some((root, end)) = v.rsplit_once(':') {
            println!(">>> {} <<<", end);
        }
        // } else {
        //     println!("NOT a path");
        // }
    }
}

/// Split a path into its constituant Basis and Dict, if the path is legal.
pub fn split_basis_and_dict<'a, F: Fn() -> Option<&'a str>>(
    src: &'a str,
    default: F,
) -> io::Result<(Option<&'a str>, Option<&'a str>)> {
    let mut basis = None;
    let dict;
    if let Some(src) = src.strip_prefix(MAIN_SEPARATOR) {
        if let Some((maybe_basis, maybe_dict)) = src.split_once(MAIN_SEPARATOR) {
            if !maybe_basis.is_empty() {
                println!("basis is empty");
                basis = Some(maybe_basis);
            } else {
                println!("default basis");
                basis = default();
            }

            if maybe_dict.is_empty() {
                println!("dict is empty");
                dict = None;
            } else {
                println!("Nope, dict not empty");
                dict = Some(maybe_dict);
            }
        } else {
            if !src.is_empty() {
                println!("Oh, src is not empty?");
                basis = Some(src);
            } else {
                println!("it's empty?")
            }
            dict = None;
        }
    } else {
        if src.is_empty() {
            println!("Short circuit");
            return Ok((basis, Some("")));
        }
        println!("other thing -- src is {}", src);
        dict = Some(src);
    }

    if let Some(basis) = &basis {
        if basis.ends_with(MAIN_SEPARATOR) {
            return Err(io::Error::new(io::ErrorKind::Other, "invalid path"));
        }
    }
    if let Some(dict) = &dict {
        if dict.ends_with(MAIN_SEPARATOR) {
            return Err(io::Error::new(io::ErrorKind::Other, "invalid path"));
        }
    }
    Ok((basis, dict))
}

#[cfg(test)]
fn default_path<'a>() -> Option<&'a str> {
    Some("{DEFAULT}")
}

#[test]
fn empty_string() {
    assert_eq!(
        split_basis_and_dict("", default_path).unwrap(),
        (None, Some(""))
    );
}

#[test]
fn bare_dict() {
    assert_eq!(
        split_basis_and_dict("one", default_path).unwrap(),
        (None, Some("one"))
    );
}

#[test]
fn dict_with_colon() {
    assert_eq!(
        split_basis_and_dict("one:two", default_path).unwrap(),
        (None, Some("one:two"))
    );
}

#[test]
fn dict_with_two_colons() {
    assert_eq!(
        split_basis_and_dict("one:two:three", default_path).unwrap(),
        (None, Some("one:two:three"))
    );
}

#[test]
#[should_panic]
fn dict_with_trailing_colon() {
    split_basis_and_dict("one:", default_path).unwrap();
}

#[test]
#[should_panic]
fn two_dicts_with_trailing_colon() {
    split_basis_and_dict("one:two:", default_path).unwrap();
}

#[test]
#[should_panic]
fn basis_with_dict_with_trailing_colon() {
    split_basis_and_dict(":one:two:", default_path).unwrap();
}

#[test]
#[should_panic]
fn basis_with_two_dicts_with_trailing_colon() {
    split_basis_and_dict(":one:two:three:", default_path).unwrap();
}

#[test]
fn basis_missing_colon() {
    assert_eq!(
        split_basis_and_dict(":one", default_path).unwrap(),
        (Some("one"), None)
    );
}

#[test]
fn basis_with_one_dict() {
    assert_eq!(
        split_basis_and_dict(":one:two", default_path).unwrap(),
        (Some("one"), Some("two"))
    );
}

#[test]
fn basis_with_two_dicts() {
    assert_eq!(
        split_basis_and_dict(":one:two:three", default_path).unwrap(),
        (Some("one"), Some("two:three"))
    );
}
#[test]
fn double_colon() {
    let default = default_path();
    assert_eq!(
        split_basis_and_dict("::", default_path).unwrap(),
        (default, None)
    );
}

#[test]
fn single_colon() {
    assert_eq!(
        split_basis_and_dict(":", default_path).unwrap(),
        (None, None)
    );
}

#[test]
fn double_colon_one_key() {
    let default = default_path();
    assert_eq!(
        split_basis_and_dict("::foo", default_path).unwrap(),
        (default, Some("foo"))
    );
}

#[test]
fn double_colon_one_key_no_default() {
    let default_path = || None;
    assert_eq!(
        split_basis_and_dict("::foo", default_path).unwrap(),
        (None, Some("foo"))
    );
}

#[test]
fn double_colon_two_keys() {
    let default = default_path();
    assert_eq!(
        split_basis_and_dict("::foo:bar", default_path).unwrap(),
        (default, Some("foo:bar"))
    );
}

#[test]
fn double_colon_three_keys() {
    let default = default_path();
    assert_eq!(
        split_basis_and_dict("::foo:bar:baz", default_path).unwrap(),
        (default, Some("foo:bar:baz"))
    );
}

#[test]
#[should_panic]
fn double_colon_three_keys_trailing_colon() {
    split_basis_and_dict("::foo:bar:baz:", default_path).unwrap();
}

#[test]
#[should_panic]
fn dict_with_two_keys_two_trailing_colons() {
    split_basis_and_dict("foo:bar::", default_path).unwrap();
}

#[test]
#[should_panic]
fn dict_with_two_keys_three_trailing_colons() {
    split_basis_and_dict("foo:bar:::", default_path).unwrap();
}
