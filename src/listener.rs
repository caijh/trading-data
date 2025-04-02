use crate::token::token_job::SyncHKEXTokenJob;
use crate::token::token_svc;
use application_beans::factory::bean_factory::{BeanFactory, ConfigurableBeanFactory};
use application_boot::application::{Application, RustApplication};
use application_boot::application_listener::ApplicationListener;
use application_context::context::application_context::APPLICATION_CONTEXT;
use application_context::context::application_event::{ApplicationEvenType, ApplicationEvent};
use application_core::env::property_resolver::PropertyResolver;
use application_schedule::scheduler::Scheduler;
use async_trait::async_trait;
use database_common::connection::DbConnection;
use database_mysql_seaorm::Dao;
use redis_io::{Redis, RedisConfig};
use std::error::Error;

pub struct ApplicationContextInitializedListener {}

#[async_trait]
impl ApplicationListener for ApplicationContextInitializedListener {
    fn is_support(&self, event: &dyn ApplicationEvent) -> bool {
        event.get_event_type() == ApplicationEvenType::ContextInitialized
    }

    async fn on_application_event(
        &self,
        application: &RustApplication,
        _event: &dyn ApplicationEvent,
    ) -> Result<(), Box<dyn Error>> {
        let application_context = application.get_application_context().await;
        let environment = application_context.get_environment().await;
        let db_connection = environment
            .get_property::<DbConnection>("database")
            .unwrap();
        let database_service = Dao::new(db_connection).await;
        application_context.get_bean_factory().set(database_service);

        let redis_config = environment.get_property::<RedisConfig>("redis");
        if redis_config.is_some() {
            Redis::init(&redis_config.unwrap())
        }
        Ok(())
    }
}

pub struct ApplicationStartedEventListener {}

#[async_trait]
impl ApplicationListener for ApplicationStartedEventListener {
    fn is_support(&self, event: &dyn ApplicationEvent) -> bool {
        event.get_event_type() == ApplicationEvenType::Started
    }

    async fn on_application_event(
        &self,
        _application: &RustApplication,
        _event: &dyn ApplicationEvent,
    ) -> Result<(), Box<dyn Error>> {
        token_svc::reset_hkex_token().await?;

        load_jobs().await?;

        Ok(())
    }
}

async fn load_jobs() -> Result<(), Box<dyn Error>> {
    let scheduler = Scheduler::new().await?;

    let application_context = APPLICATION_CONTEXT.read().await;
    application_context.get_bean_factory().set(scheduler);
    let scheduler = application_context.get_bean_factory().get::<Scheduler>();
    scheduler.start().await?;

    let _ = scheduler
        .add_job(
            1,
            "同步HKEX的AccessToken",
            "0 0 9,12,15 * * *",
            Box::new(SyncHKEXTokenJob),
        )
        .await;

    Ok(())
}
