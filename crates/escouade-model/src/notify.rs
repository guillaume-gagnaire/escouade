//! What a notification is about (`agent::NotifyKind`): each kind has its own choice in the settings.

/// What a notification is about: each kind has its own choice in the settings ("Me prévenir pour").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyKind {
    /// A question or a permission waits for the user.
    Question,
    /// A turn ended.
    Done,
    /// A turn failed, or the process died.
    Error,
    /// A ticket is ready to test, or blocked (raised by the board, never by an agent's own turn).
    Ticket,
    /// What a reading of a pull request or a deployment tells (ESC-68): the host's news, not an
    /// agent's.
    PrCiFailed,
    PrChanges,
    PrApproved,
    PrMerged,
    PrConflict,
    PrReview,
    DeployOk,
    DeployFailed,
}

impl NotifyKind {
    /// Every kind, for the tests of the choices.
    #[cfg(any(test, feature = "testing"))]
    pub const ALL: [Self; 12] = [
        Self::Question,
        Self::Done,
        Self::Error,
        Self::Ticket,
        Self::PrCiFailed,
        Self::PrChanges,
        Self::PrApproved,
        Self::PrMerged,
        Self::PrConflict,
        Self::PrReview,
        Self::DeployOk,
        Self::DeployFailed,
    ];

    /// The news of a code host: switched off, it says nothing at all (no agent waits behind it).
    pub fn of_host(self) -> bool {
        !matches!(
            self,
            Self::Question | Self::Done | Self::Error | Self::Ticket
        )
    }
}
