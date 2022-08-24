use bstr::{BStr, BString, ByteSlice};

/// The kind of helper program to use.
pub enum Kind {
    /// The built-in git-credential helper program, part of any git distribution.
    GitCredential,
}

/// Additional context to be passed to the credentials helper.
// TODO: fill in what's needed per configuration
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Context {
    /// The protocol over which the credential will be used (e.g., https).
    pub protocol: Option<String>,
    /// The remote hostname for a network credential. This includes the port number if one was specified (e.g., "example.com:8088").
    pub host: Option<String>,
    /// The path with which the credential will be used. E.g., for accessing a remote https repository, this will be the repository’s path on the server.
    /// It can also be a path on the file system.
    pub path: Option<BString>,
    /// The credential’s username, if we already have one (e.g., from a URL, the configuration, the user, or from a previously run helper).
    pub username: Option<String>,
    /// The credential’s password, if we are asking it to be stored.
    pub password: Option<String>,
    /// When this special attribute is read by git credential, the value is parsed as a URL and treated as if its constituent
    /// parts were read (e.g., url=<https://example.com> would behave as if
    /// protocol=https and host=example.com had been provided). This can help callers avoid parsing URLs themselves.
    pub url: Option<BString>,
}
///
pub mod context;

/// The action to perform by the credentials [helper][`crate::helper()`].
#[derive(Clone, Debug)]
pub enum Action {
    /// Provide credentials using the given repository context, which must include the repository url.
    Get(Context),
    /// Approve the credentials as identified by the previous input provided as `BString`, containing information from [`Context`].
    Store(BString),
    /// Reject the credentials as identified by the previous input provided as `BString`. containing information from [`Context`].
    Erase(BString),
}

/// Initialization
impl Action {
    /// Create a `Get` action with context containing the given URL
    pub fn get_for_url(url: impl Into<BString>) -> Action {
        Action::Get(Context {
            url: Some(url.into()),
            ..Default::default()
        })
    }
}

/// Access
impl Action {
    /// Return the payload of store or erase actions.
    pub fn payload(&self) -> Option<&BStr> {
        match self {
            Action::Get(_) => None,
            Action::Store(p) | Action::Erase(p) => Some(p.as_bstr()),
        }
    }
    /// Returns true if this action expects output from the helper.
    pub fn expects_output(&self) -> bool {
        matches!(self, Action::Get(_))
    }
    /// The name of the argument to describe this action. If `is_custom` is true, the target program is
    /// a custom credentials helper, not a built-in one.
    pub fn as_helper_arg(&self, is_custom: bool) -> &str {
        match self {
            Action::Get(_) if is_custom => "get",
            Action::Get(_) => "fill",
            Action::Store(_) if is_custom => "store",
            Action::Store(_) => "approve",
            Action::Erase(_) if is_custom => "erase",
            Action::Erase(_) => "reject",
        }
    }
}

/// A handle to [store][NextAction::store()] or [erase][NextAction::erase()] the outcome of the initial action.
#[derive(Clone, Debug)]
pub struct NextAction {
    previous_output: BString,
}

impl NextAction {
    /// Approve the result of the previous [Action] and store for lookup.
    pub fn store(self) -> Action {
        Action::Store(self.previous_output)
    }
    /// Reject the result of the previous [Action] and erase it as to not be returned when being looked up.
    pub fn erase(self) -> Action {
        Action::Erase(self.previous_output)
    }
}

///
pub mod invoke;
pub use invoke::function::invoke;
