//! Infrastructure adapters for the Work bounded context.

pub mod clock_id;
pub mod command_result_store;
pub mod config;
pub mod idempotency_store;
pub mod outbox_store;
pub mod repositories;
pub mod runtime_builder;
pub mod source_resolvers;

use core_contracts as _;
use work_application as _;
use work_contracts as _;
use work_domain as _;
