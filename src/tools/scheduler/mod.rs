//! Scheduler tools — wrappers around SchedulerService.

pub mod cancel_scheduled;
pub mod list_scheduled;
pub mod schedule_one_time;
pub mod schedule_recurring;

pub fn register_all(registry: &crate::tools::registry::ToolRegistry) {
    registry.register(schedule_one_time::ScheduleOneTimeTool);
    registry.register(schedule_recurring::ScheduleRecurringTool);
    registry.register(list_scheduled::ListScheduledTool);
    registry.register(cancel_scheduled::CancelScheduledTool);
}
