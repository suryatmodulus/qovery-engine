use crate::cloud_provider::environment::Environment;
use crate::cloud_provider::CloudProvider;
use crate::container_registry::ContainerRegistry;
use crate::io_models::application::Application;
use crate::io_models::container::Container;
use crate::io_models::context::Context;
use crate::io_models::database::Database;
use crate::io_models::job::Job;
use crate::io_models::router::Router;
use crate::io_models::Action;
use crate::models::application::ApplicationError;
use crate::models::container::ContainerError;
use crate::models::database::DatabaseError;
use crate::models::job::JobError;
use crate::models::router::RouterError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct EnvironmentRequest {
    pub execution_id: String,
    pub long_id: Uuid,
    pub name: String,
    pub project_long_id: Uuid,
    pub organization_long_id: Uuid,
    pub action: Action,
    pub applications: Vec<Application>,
    pub containers: Vec<Container>,
    pub jobs: Vec<Job>,
    pub routers: Vec<Router>,
    pub databases: Vec<Database>,
}

#[derive(thiserror::Error, Debug)]
pub enum DomainError {
    #[error("Invalid application: {0}")]
    ApplicationError(ApplicationError),
    #[error("Invalid container: {0}")]
    ContainerError(ContainerError),
    #[error("Invalid router: {0}")]
    RouterError(RouterError),
    #[error("Invalid database: {0}")]
    DatabaseError(DatabaseError),
    #[error("Invalid job: {0}")]
    JobError(JobError),
}

impl EnvironmentRequest {
    pub fn to_environment_domain(
        &self,
        context: &Context,
        cloud_provider: &dyn CloudProvider,
        container_registry: &dyn ContainerRegistry,
    ) -> Result<Environment, DomainError> {
        let mut applications = Vec::with_capacity(self.applications.len());
        for app in &self.applications {
            match app.clone().to_application_domain(
                context,
                app.to_build(container_registry.registry_info()),
                cloud_provider,
            ) {
                Ok(app) => applications.push(app),
                Err(err) => {
                    return Err(DomainError::ApplicationError(err));
                }
            }
        }

        let mut containers = Vec::with_capacity(self.containers.len());
        for container in &self.containers {
            match container
                .clone()
                .to_container_domain(context, cloud_provider, container_registry)
            {
                Ok(app) => containers.push(app),
                Err(err) => {
                    return Err(DomainError::ContainerError(err));
                }
            }
        }

        let mut routers = Vec::with_capacity(self.routers.len());
        for router in &self.routers {
            let mut custom_domain_check_enabled = true;
            let mut whitelist_source_range = "0.0.0.0/0".to_string();

            for app in &self.applications {
                for route in &router.routes {
                    if route.service_long_id == app.long_id {
                        if !app.advanced_settings.deployment_custom_domain_check_enabled {
                            // disable custom domain check for this router
                            custom_domain_check_enabled = false;
                        }

                        whitelist_source_range = app.advanced_settings.network_ingress_whitelist_source_range.clone();
                    }
                }
            }

            for container in &self.containers {
                for route in &router.routes {
                    if route.service_long_id == container.long_id {
                        if !container.advanced_settings.deployment_custom_domain_check_enabled {
                            // disable custom domain check for this router
                            custom_domain_check_enabled = false;
                        }

                        whitelist_source_range = container
                            .advanced_settings
                            .network_ingress_whitelist_source_range
                            .clone();
                    }
                }
            }

            match router.to_router_domain(context, custom_domain_check_enabled, whitelist_source_range, cloud_provider)
            {
                Ok(router) => routers.push(router),
                Err(err) => {
                    return Err(DomainError::RouterError(err));
                }
            }
        }

        let mut databases = Vec::with_capacity(self.databases.len());
        for db in &self.databases {
            match db.to_database_domain(context, cloud_provider) {
                Ok(router) => databases.push(router),
                Err(err) => {
                    return Err(DomainError::DatabaseError(err));
                }
            }
        }

        let mut jobs = Vec::with_capacity(self.jobs.len());
        for job in &self.jobs {
            match job.clone().to_job_domain(context, cloud_provider, container_registry) {
                Ok(job) => jobs.push(job),
                Err(err) => return Err(DomainError::JobError(err)),
            }
        }

        Ok(Environment::new(
            self.long_id,
            self.name.clone(),
            self.project_long_id,
            self.organization_long_id,
            self.action.to_service_action(),
            context,
            applications,
            containers,
            routers,
            databases,
            jobs,
        ))
    }
}
