//! The inactive state of an application and its boolean form.

/// Whether Android marks an application as inactive.
///
/// `am set-inactive` takes this state as a boolean, so `inactive` maps to
/// `true` and `active` maps to `false`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InactiveState {
    /// The application is not marked as inactive.
    Active,
    /// The application is marked as inactive.
    Inactive,
}

impl InactiveState {
    /// Returns the value shown in the application list.
    pub fn name(self) -> &'static str {
        let name = match self {
            InactiveState::Active => "active",
            InactiveState::Inactive => "inactive",
        };
        return name;
    }

    /// Reads a state from the value shown in the application list.
    pub fn from_name(name: &str) -> Option<Self> {
        let state = match name {
            "active" => InactiveState::Active,
            "inactive" => InactiveState::Inactive,
            _ => return None,
        };
        return Some(state);
    }

    /// Returns the boolean that `am set-inactive` accepts for this state.
    pub fn flag(self) -> &'static str {
        let flag = match self {
            InactiveState::Active => "false",
            InactiveState::Inactive => "true",
        };
        return flag;
    }

    /// Reads a state from the boolean that Android prints.
    ///
    /// Any other value yields [`None`], so the user sees that the state is
    /// unknown instead of a guess.
    pub fn from_flag(flag: &str) -> Option<Self> {
        if flag.eq_ignore_ascii_case("true") {
            return Some(InactiveState::Inactive);
        }
        if flag.eq_ignore_ascii_case("false") {
            return Some(InactiveState::Active);
        }
        return None;
    }

    /// Returns the other state: `active` becomes `inactive` and back.
    pub fn toggled(self) -> Self {
        let state = match self {
            InactiveState::Active => InactiveState::Inactive,
            InactiveState::Inactive => InactiveState::Active,
        };
        return state;
    }
}
