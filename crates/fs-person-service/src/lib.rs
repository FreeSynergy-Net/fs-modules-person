//! `fs-person-service`: the person module as a service (one process per person). Empty skeleton; no code yet.
//!
//! Pattern: Adapter – the service turns the calls of the form source (fs-rpc) into calls of the port of
//! `fs-person`; the transport is the adapter, the module does not know it.
