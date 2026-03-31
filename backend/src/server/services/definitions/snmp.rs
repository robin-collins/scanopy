use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{ClientProbe, Pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Snmp;

impl ServiceDefinition for Snmp {
    fn name(&self) -> &'static str {
        "SNMP"
    }
    fn description(&self) -> &'static str {
        "Simple Network Management Protocol"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::SNMP
    }
    fn discovery_pattern(&self) -> Pattern<'_> {
        // SNMP port detection is credential-gated (scan_udp_ports uses
        // try_snmp_with_credential_on_port), so Pattern::Port(Snmp) was
        // already implicitly credential-gated. ClientResponse makes this explicit
        // and consistent with Docker's pattern.
        Pattern::ClientResponse(ClientProbe::Snmp)
    }
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Snmp>));
