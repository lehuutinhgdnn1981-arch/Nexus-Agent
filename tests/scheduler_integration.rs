//! Integration tests cho scheduler.

mod common;

use nexus::scheduler::job::{JobKind, JobSpec};
use nexus::scheduler::nlp::parse_natural_language;
use nexus::scheduler::persistence;
use nexus::scheduler::SchedulerService;
use chrono::{Duration, Utc};
use std::sync::Arc;

#[tokio::test]
async fn integration_scheduler_persist_and_restore() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    // Save 2 jobs
    let j1 = JobSpec::one_time("job-1", Utc::now() + Duration::hours(1), "remind 1");
    let j2 = JobSpec::recurring("job-2", "0 9 * * *", "daily standup");

    persistence::save_job(&ctx.pool, &j1).await.unwrap();
    persistence::save_job(&ctx.pool, &j2).await.unwrap();

    // Load enabled
    let loaded = persistence::load_enabled_jobs(&ctx.pool).await.unwrap();
    assert_eq!(loaded.len(), 2);

    // Disable j1
    persistence::disable_job(&ctx.pool, "job-1").await.unwrap();
    let loaded = persistence::load_enabled_jobs(&ctx.pool).await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "job-2");

    // Get single
    let j1_loaded = persistence::get_job(&ctx.pool, "job-1").await.unwrap();
    assert!(j1_loaded.is_some());
    assert_eq!(j1_loaded.unwrap().id, "job-1");

    // Delete
    persistence::delete_job(&ctx.pool, "job-2").await.unwrap();
    let all = persistence::list_all_jobs(&ctx.pool).await.unwrap();
    assert_eq!(all.len(), 1); // only j1 remains (disabled)
}

#[tokio::test]
async fn integration_scheduler_service_add_and_list() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let on_fire: nexus::scheduler::service::FireCallback = Arc::new(|_, _, _| {});
    let svc = SchedulerService::new(ctx.pool.clone(), on_fire);
    svc.start().await.unwrap();

    let id1 = svc
        .add_from_natural_language("in 1 hour", "do something", None)
        .await
        .unwrap();
    let id2 = svc
        .add_from_natural_language("every day 9am", "morning reminder", None)
        .await
        .unwrap();

    let jobs = svc.list().await.unwrap();
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().any(|j| j.id == id1));
    assert!(jobs.iter().any(|j| j.id == id2));

    svc.cancel(&id1).await.unwrap();
    let jobs = svc.list().await.unwrap();
    assert_eq!(jobs.len(), 2); // cancel disables but doesn't delete

    svc.shutdown().await.unwrap();
}

#[test]
fn integration_nlp_one_time_patterns() {
    let cases = vec![
        "in 30 minutes",
        "in 2 hours",
        "in 1 day",
        "tomorrow 9am",
        "tomorrow 9:30am",
    ];
    for c in cases {
        let kind = parse_natural_language(c).unwrap_or_else(|e| panic!("`{c}` failed: {e}"));
        match kind {
            JobKind::OneTime { fire_at } => {
                assert!(fire_at > Utc::now(), "fire_at must be in future for `{c}`");
            }
            JobKind::Recurring { .. } => panic!("`{c}` should be one-time"),
        }
    }
}

#[test]
fn integration_nlp_recurring_patterns() {
    let cases = vec![
        ("every day 9am", "0 9 * * *"),
        ("every weekday 8:30am", "30 8 * * 1-5"),
        ("every monday 10am", "0 10 * * 1"),
        ("every hour", "0 * * * *"),
        ("every 30 minutes", "*/30 * * * *"),
    ];
    for (input, expected_cron) in cases {
        let kind = parse_natural_language(input).unwrap_or_else(|e| panic!("`{input}` failed: {e}"));
        match kind {
            JobKind::Recurring { cron } => assert_eq!(cron, expected_cron, "for `{input}`"),
            _ => panic!("`{input}` should be recurring"),
        }
    }
}

#[test]
fn integration_nlp_invalid_input() {
    assert!(parse_natural_language("").is_err());
    assert!(parse_natural_language("garbage input").is_err());
    assert!(parse_natural_language("in xyz hours").is_err());
}
