use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Windows;

impl ServiceDefinition for Windows {
    fn name(&self) -> &'static str {
        "Windows"
    }
    fn description(&self) -> &'static str {
        "Windows Remote Management (WinRM) remote access"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::RemoteAccess
    }
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Port(PortType::WinRm),
            Pattern::Port(PortType::WinRmHttps),
            Pattern::ClientResponse(crate::server::services::r#impl::patterns::ClientProbe::WinRm),
        ])
    }
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Windows>));
