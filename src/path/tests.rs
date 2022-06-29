use super::MAIN_SEP;
use std::io;

fn split_basis_and_dict<F: Fn() -> Option<String>>(src: &str, default: F) -> io::Result<(Option<String>, Option<String>)> {
    let mut basis = None;
    let dict;
    if let Some(src) = src.strip_prefix(MAIN_SEP) {
        if let Some((maybe_basis, maybe_dict)) = src.split_once(MAIN_SEP) {
            if !maybe_basis.is_empty() {
                basis = Some(maybe_basis.to_owned());
            } else {
                basis = default();
            }

            if maybe_dict.is_empty() {
                dict = None;
            } else {
                dict = Some(maybe_dict.to_owned());
            }
        } else {
            if !src.is_empty() {
                basis = Some(src.to_owned());
            }
            dict = None;
        }
    } else {
        if src.is_empty() {
            return Err(io::Error::new(io::ErrorKind::Other, "something's fishy"));
        }
        dict = Some(src.to_owned());
    }

    if let Some(basis) = &basis {
        if basis.ends_with(MAIN_SEP) {
            return Err(io::Error::new(io::ErrorKind::Other, "invalid path"));
        }
    }
    if let Some(dict) = &dict {
        if dict.ends_with(MAIN_SEP) {
            return Err(io::Error::new(io::ErrorKind::Other, "invalid path"));
        }
    }
    Ok((basis, dict))
}

#[cfg(test)]
fn default_path() -> Option<String> {
    Some("{DEFAULT}".to_owned())
}

#[test]
#[should_panic]
fn empty_string() {
    split_basis_and_dict("", default_path).unwrap();
}

#[test]
fn bare_dict() {
    assert_eq!(split_basis_and_dict("one", default_path).unwrap(), (None, Some("one".to_owned())));
}

#[test]
fn dict_with_colon() {
    assert_eq!(split_basis_and_dict("one:two", default_path).unwrap(), (None, Some("one:two".to_owned())));
}

#[test]
fn dict_with_two_colons() {
    assert_eq!(split_basis_and_dict("one:two:three", default_path).unwrap(), (None, Some("one:two:three".to_owned())));
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
    assert_eq!(split_basis_and_dict(":one", default_path).unwrap(), (Some("one".to_owned()), None));
}

#[test]
fn basis_with_one_dict() {
    assert_eq!(split_basis_and_dict(":one:two", default_path).unwrap(), (Some("one".to_owned()), Some("two".to_owned())));
}

#[test]
fn basis_with_two_dicts() {
    assert_eq!(split_basis_and_dict(":one:two:three", default_path).unwrap(), (Some("one".to_owned()), Some("two:three".to_owned())));
}
#[test]
fn double_colon() {
    let default = default_path();
    assert_eq!(split_basis_and_dict("::", default_path).unwrap(), (default, None));
}

#[test]
fn single_colon() {
    assert_eq!(split_basis_and_dict(":", default_path).unwrap(), (None, None));
}

#[test]
fn double_colon_two_keys() {
    let default = default_path();
    assert_eq!(split_basis_and_dict("::foo:bar", default_path).unwrap(), (default, Some("foo:bar".to_owned())));
}

#[test]
fn double_colon_three_keys() {
    let default = default_path();
    assert_eq!(split_basis_and_dict("::foo:bar:baz", default_path).unwrap(), (default, Some("foo:bar:baz".to_owned())));
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