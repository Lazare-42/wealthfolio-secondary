//! Activity provenance / import traceability.
//!
//! Records where an imported activity came from (email, pdf, csv, bank export,
//! manual, chat) and, optionally, which other activity funded it — e.g. a SELL
//! whose proceeds funded a LOAN_ORIGINATION (a compte courant d'associé). Also
//! snapshots the transaction emails a chat session chose as the source.

pub mod provenance_model;
pub mod provenance_service;
pub mod provenance_traits;

pub use provenance_model::{
    ActivitySource, ChatSourceEmail, NewActivitySource, NewChatSourceEmail, SourceKind,
};
pub use provenance_service::ProvenanceService;
pub use provenance_traits::{ProvenanceRepositoryTrait, ProvenanceServiceTrait};
