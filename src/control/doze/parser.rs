//! Parsing the complete Device Idle whitelist.

use std::fmt;

use super::state::Whitelist;

/// How Android labels one lasting whitelist entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Category {
    /// A package that the user whitelisted.
    User,
    /// A package that Android manages through a system whitelist.
    System,
    /// A package that is only whitelisted until a timeout.
    Temporary,
}

impl Category {
    /// Reads the category that Android prints for an entry.
    ///
    /// Both `system` and `system-excidle` are system-managed categories, and
    /// the temporary categories are recognized so that they can be ignored
    /// instead of ending up in either lasting whitelist.
    fn from_name(name: &str) -> Option<Self> {
        let category = match name {
            "user" => Category::User,
            "system" | "system-excidle" => Category::System,
            "temp" | "tempwhitelist" => Category::Temporary,
            _ => return None,
        };
        return Some(category);
    }
}

/// One line of whitelist output that could not be understood.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct WhitelistError {
    line: usize,
    text: String,
}

impl fmt::Display for WhitelistError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        return write!(
            formatter,
            "line {} is not a whitelist entry: `{}`",
            self.line, self.text
        );
    }
}

impl std::error::Error for WhitelistError {}

/// Parses the output of `cmd deviceidle whitelist`.
///
/// Every non-empty line is `category,package,uid`. Temporary entries are
/// ignored, because they are not part of either lasting whitelist. A line that
/// is not an entry, or that carries a category this application does not know,
/// is reported as an error instead of being dropped, so an unexpected answer
/// never turns into a confident classification of the installed applications.
pub fn parse_whitelist(output: &str) -> Result<Whitelist, WhitelistError> {
    let mut whitelist = Whitelist::default();

    for (index, line) in output.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let (category, package) = parse_entry(line, index + 1)?;
        match category {
            Category::System => whitelist.add_system(package),
            Category::User => whitelist.add_user(package),
            Category::Temporary => {}
        }
    }
    return Ok(whitelist);
}

/// Reads the category and package of one whitelist line.
fn parse_entry(
    line: &str,
    number: usize,
) -> Result<(Category, &str), WhitelistError> {
    let mut fields = line.split(',');
    let category = fields.next();
    let package = fields.next();
    let _uid = fields.next();
    let extra = fields.next();

    let (category, package) = match (category, package) {
        (Some(category), Some(package)) => (category.trim(), package.trim()),
        _ => return Err(malformed(number, line)),
    };
    if package.is_empty() || extra.is_some() {
        return Err(malformed(number, line));
    }

    let category = match Category::from_name(category) {
        Some(category) => category,
        None => return Err(malformed(number, line)),
    };
    return Ok((category, package));
}

/// Builds the error of one unreadable whitelist line.
fn malformed(number: usize, line: &str) -> WhitelistError {
    let error = WhitelistError {
        line: number,
        text: String::from(line),
    };
    return error;
}
