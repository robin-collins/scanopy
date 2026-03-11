use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct SaltProxy;

impl ServiceDefinition for saltproxy {
    fn name(&self) -> &'static str {
        "Salt Proxy"
    }
    fn description(&self) -> &'static str {
        "Salt Proxy"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkCore
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
        Pattern::Port(PortBase::new_tcp(4505)),
        Pattern::Port(PortBase::new_tcp(4506)),
        Pattern::Port(PortBase::new_tcp(8022)),
    ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/salt-project.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<SaltProxy>));
