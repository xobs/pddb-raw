/// # PDDB Path Conventions
///
/// A PDDB Path may be a dict, a dict + a key, a basis + dict, or a basis + dict + key.
/// In the following examples, the given Basis, Dict, and Key are as follows:
///
/// * Basis: `.System`
/// * Dict: `wlan.networks`
/// * Key: `Home Wifi`
///
/// A canonical path looks like:
///
///     [:BASIS:]DICT1:DICT2:DICT3[:KEY]
///
/// Examples:
///
/// * `:Home Wifi` -- A basis named "Home Wifi"
/// * `:.System:` -- A basis named ".System"
/// * `wlan.networks` -- A dict named "wlan.networks" in the default basis
/// * `wlan.networks:recent` -- A dict named "wlan.networks:recent", which may be considered a path, in the default basis. This also desecribes a key called "recent" in the dict "wlan.networks", depending on whether you're treating it as a directory or a file.
/// * `:.System:wlan.networks` -- A dict named "wlan.networks" in the basis ".System"
/// * `:.System:wlan.networks:recent` -- a fully-qualified path, describing a key "recent" in the dict "wlan.networks" in the basis ".System". Also describes a dict "wlan.networks:recent" in the basis ".System" when treating it as a directory.
/// * `:` -- The root, which lists every basis. Files cannot be created here. "Directories" can be
///             created and destroyed, which corresponds to creating and destroying bases.
/// * `::` -- An empty basis is a synonym for all bases, so this corresponds to listing all dicts in the root of the default basis.
/// *  -- An empty string corresponds to listing all dicts in root the union basis.
///
/// Corner cases:
///
/// * `: :` -- A basis named " ". Legal, but questionable
/// * ` ` -- A dict named " " in the default basis. Legal, but questionable.
/// * `: ` -- Also a dict named " " in the default basis.
/// * ` : ` -- A key named " " in a dict called " ". Legal.
/// * `baz:` -- A dict named "baz" in the default basis with an extra ":" following. Legal.
/// * `baz:foo:` -- Identical to "baz:foo", may be either a dict "baz:foo" or a key "foo" in the dict "baz"
/// * `:::` -- An key named ":" in an empty dict in the default basis. Illegal.
/// * `::::` -- An key named "::" in an empty dict in the default basis. Illegal.
/// * `::foo` -- A key "foo" in the default basis.
/// * `:lorem.ipsum:foo:baz` -- A key called "foo:baz" in the basis "lorem.ipsum". May also describe a dict "foo:baz" in the basis "lorem.ipsum" if treated as a directory.
/// * `:bar:lorem.ipsum:foo:baz` -- A key called "baz" in the dict "lorem.ipsum:foo" in
///             the basis "bar", or a dict called "lorem.ipsum:foo:baz". Legal.
///
/// Any reference to "default basis" depends on whether the operation is a "read" or a "write":
///
/// * "Read" operations come from a union, with the most-recently-added basis taking precedence
/// * "Write" operations go to the most-recently-added basis that contains the key. If the key does not exist and "Create" was specified, then the file is created in the most-recently-added basis.#[cfg(test)]
#[cfg(test)]
mod tests;

#[allow(unused)]
pub const MAIN_SEP_STR: &str = ":";
#[allow(unused)]
pub const MAIN_SEP: char = ':';

#[inline]
#[allow(unused)]
pub fn is_sep_byte(b: u8) -> bool {
    b == b':'
}

#[inline]
#[allow(unused)]
pub fn is_verbatim_sep(b: u8) -> bool {
    b == b':'
}

