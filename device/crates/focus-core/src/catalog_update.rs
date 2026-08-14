use crate::{Catalog, InputEvent};

pub const CATALOG_CONFIRMATION_TIMEOUT_MS: u64 = 15_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogStageError {
    Busy,
    Conflict { current_revision: u64 },
    DeadlineOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogStageOutcome {
    pub proposal_id: u32,
    pub replaced_proposal_id: Option<u32>,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogCommit {
    pub proposal_id: u32,
    pub expected_revision: u64,
    pub catalog: Catalog,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum CatalogConfirmationAction {
    None,
    Commit(CatalogCommit),
    Rejected { proposal_id: u32 },
    Expired { proposal_id: u32 },
    Cancelled { proposal_id: u32 },
}

#[derive(Clone, Debug)]
struct PendingCatalogUpdate {
    proposal_id: u32,
    expected_revision: u64,
    catalog: Catalog,
    expires_at_ms: u64,
}

/// Pure, fixed-capacity coordinator for the one physical confirmation prompt.
#[derive(Clone, Debug, Default)]
pub struct CatalogUpdateCoordinator {
    pending: Option<PendingCatalogUpdate>,
}

impl CatalogUpdateCoordinator {
    #[must_use]
    pub const fn new() -> Self {
        Self { pending: None }
    }

    #[must_use]
    pub fn pending_proposal_id(&self) -> Option<u32> {
        self.pending.as_ref().map(|pending| pending.proposal_id)
    }

    #[must_use]
    pub fn pending_entry_count(&self) -> Option<usize> {
        self.pending
            .as_ref()
            .map(|pending| pending.catalog.len().saturating_sub(5))
    }

    /// Stages a complete replacement behind the physical confirmation gate.
    ///
    /// # Errors
    ///
    /// Returns busy, optimistic revision conflict, or monotonic deadline overflow.
    pub fn stage(
        &mut self,
        now_ms: u64,
        app_is_idle: bool,
        current_revision: u64,
        expected_revision: u64,
        proposal_id: u32,
        catalog: Catalog,
    ) -> Result<CatalogStageOutcome, CatalogStageError> {
        if !app_is_idle {
            return Err(CatalogStageError::Busy);
        }
        if expected_revision != current_revision {
            return Err(CatalogStageError::Conflict { current_revision });
        }
        let expires_at_ms = now_ms
            .checked_add(CATALOG_CONFIRMATION_TIMEOUT_MS)
            .ok_or(CatalogStageError::DeadlineOverflow)?;
        let replaced_proposal_id = self.pending.as_ref().map(|pending| pending.proposal_id);
        self.pending = Some(PendingCatalogUpdate {
            proposal_id,
            expected_revision,
            catalog,
            expires_at_ms,
        });
        Ok(CatalogStageOutcome {
            proposal_id,
            replaced_proposal_id,
            expires_at_ms,
        })
    }

    pub fn handle_input(&mut self, now_ms: u64, input: InputEvent) -> CatalogConfirmationAction {
        if let Some(action) = self.expire(now_ms) {
            return action;
        }
        match input {
            InputEvent::Press => {
                self.pending
                    .take()
                    .map_or(CatalogConfirmationAction::None, |pending| {
                        CatalogConfirmationAction::Commit(CatalogCommit {
                            proposal_id: pending.proposal_id,
                            expected_revision: pending.expected_revision,
                            catalog: pending.catalog,
                        })
                    })
            }
            InputEvent::LongPress => {
                self.pending
                    .take()
                    .map_or(CatalogConfirmationAction::None, |pending| {
                        CatalogConfirmationAction::Rejected {
                            proposal_id: pending.proposal_id,
                        }
                    })
            }
            InputEvent::RotateLeft | InputEvent::RotateRight | InputEvent::TimeAdvanced => {
                CatalogConfirmationAction::None
            }
        }
    }

    pub fn expire(&mut self, now_ms: u64) -> Option<CatalogConfirmationAction> {
        let expired = self
            .pending
            .as_ref()
            .is_some_and(|pending| now_ms >= pending.expires_at_ms);
        if !expired {
            return None;
        }
        self.pending
            .take()
            .map(|pending| CatalogConfirmationAction::Expired {
                proposal_id: pending.proposal_id,
            })
    }

    pub fn cancel(&mut self) -> CatalogConfirmationAction {
        self.pending
            .take()
            .map_or(CatalogConfirmationAction::None, |pending| {
                CatalogConfirmationAction::Cancelled {
                    proposal_id: pending.proposal_id,
                }
            })
    }
}
