use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! new_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Debug,
        )]
        pub struct $name(pub u32);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.0)
            }
        }
    };
}

new_id!(SettlementId);
new_id!(RouteId);
new_id!(PropertyId);
new_id!(PersonId);
new_id!(CauseId);
new_id!(EventId);

/// Index into the causal event log.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct EventRecordId(pub u32);
