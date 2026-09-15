#[cfg(test)]
mod tests {
    use super::super::runtime::TaskRuntime;
    use super::super::state::TaskStatus;
    use super::super::types::{TaskOwner, TaskType};
    use crate::events::payload::{EdithPayload, TaskPayload};
    use crate::events::EventCorrelation;

    #[tokio::test]
    async fn test_task_creation_and_unique_ids() {
        let runtime = TaskRuntime::mock();
        let corr1 = EventCorrelation::default();
        let corr2 = EventCorrelation::default();

        let id1 = runtime
            .create_task(
                TaskType::Background,
                "Analyze codebase",
                corr1,
                TaskOwner::User,
            )
            .await;
        let id2 = runtime
            .create_task(
                TaskType::BrowserAgent,
                "Search docs",
                corr2,
                TaskOwner::User,
            )
            .await;

        assert_ne!(id1, id2);
        let task1 = runtime.get_task(&id1).await.expect("Task 1 must exist");
        let task2 = runtime.get_task(&id2).await.expect("Task 2 must exist");

        assert_eq!(task1.goal, "Analyze codebase");
        assert_eq!(task1.status, TaskStatus::Created);
        assert_eq!(task2.goal, "Search docs");
        assert_eq!(task2.status, TaskStatus::Created);
    }

    #[tokio::test]
    async fn test_task_lifecycle_valid_transitions() {
        let runtime = TaskRuntime::mock();
        let corr = EventCorrelation::default();
        let task_id = runtime
            .create_task(
                TaskType::DevAgent,
                "Refactor module",
                corr,
                TaskOwner::System,
            )
            .await;

        // Created -> Running
        assert!(runtime.start_task(&task_id).await.is_ok());
        let snapshot = runtime.get_task(&task_id).await.unwrap();
        assert_eq!(snapshot.status, TaskStatus::Running);
        assert!(snapshot.started_at_ms.is_some());

        // Update progress
        assert!(runtime
            .update_progress(&task_id, 1, 5, "Step 1 complete")
            .await
            .is_ok());
        let snapshot = runtime.get_task(&task_id).await.unwrap();
        assert_eq!(snapshot.progress.step, 1);
        assert_eq!(snapshot.progress.status_text, "Step 1 complete");

        // Running -> Completed
        assert!(runtime
            .complete_task(&task_id, "Successfully refactored")
            .await
            .is_ok());
        let snapshot = runtime.get_task(&task_id).await.unwrap();
        assert_eq!(snapshot.status, TaskStatus::Completed);
        assert_eq!(
            snapshot.result_summary.as_deref(),
            Some("Successfully refactored")
        );
        assert!(snapshot.completed_at_ms.is_some());

        // Events check
        let events = runtime.emitter().get_mock_events();
        assert!(events.iter().any(|e| matches!(
            &e.payload,
            EdithPayload::Task(TaskPayload::Started { .. })
        )));
        assert!(events.iter().any(|e| matches!(
            &e.payload,
            EdithPayload::Task(TaskPayload::StepProgress { step: 1, .. })
        )));
        assert!(events.iter().any(|e| matches!(
            &e.payload,
            EdithPayload::Task(TaskPayload::Finished { success: true, .. })
        )));
    }

    #[tokio::test]
    async fn test_task_lifecycle_invalid_transitions_rejected() {
        let runtime = TaskRuntime::mock();
        let corr = EventCorrelation::default();
        let task_id = runtime
            .create_task(
                TaskType::Background,
                "One-off job",
                corr,
                TaskOwner::User,
            )
            .await;

        // Attempting to complete before starting should fail (Created -> Completed is invalid)
        let invalid_complete = runtime.complete_task(&task_id, "Too soon").await;
        assert!(invalid_complete.is_err());

        // Start task: Created -> Running (valid)
        assert!(runtime.start_task(&task_id).await.is_ok());

        // Complete task: Running -> Completed (valid)
        assert!(runtime.complete_task(&task_id, "Done").await.is_ok());

        // Terminal state cannot transition back to Running
        let re_start = runtime.start_task(&task_id).await;
        assert!(re_start.is_err());

        // Terminal state cannot be cancelled
        let cancel_completed = runtime.cancel_task(&task_id, None).await;
        assert!(cancel_completed.is_err());
    }

    #[tokio::test]
    async fn test_task_cancellation_scoped_token() {
        let runtime = TaskRuntime::mock();
        let corr = EventCorrelation::default();
        let task_id = runtime
            .create_task(
                TaskType::BrowserAgent,
                "Scrape pages",
                corr,
                TaskOwner::User,
            )
            .await;

        let token = runtime
            .get_cancellation_token(&task_id)
            .await
            .expect("Token must exist");
        assert!(!token.is_cancelled());

        assert!(runtime.start_task(&task_id).await.is_ok());
        assert!(!token.is_cancelled());

        // Cancel task
        assert!(runtime
            .cancel_task(&task_id, Some("User requested stop".to_string()))
            .await
            .is_ok());

        // Token is cooperatively cancelled
        assert!(token.is_cancelled());

        let snapshot = runtime.get_task(&task_id).await.unwrap();
        assert_eq!(snapshot.status, TaskStatus::Cancelled);
        assert_eq!(snapshot.error.as_deref(), Some("User requested stop"));

        let events = runtime.emitter().get_mock_events();
        assert!(events.iter().any(|e| matches!(
            &e.payload,
            EdithPayload::Task(TaskPayload::Cancelled { reason, .. }) if reason.as_deref() == Some("User requested stop")
        )));
    }

    #[tokio::test]
    async fn test_concurrent_task_isolation() {
        let runtime = TaskRuntime::mock();

        let id_a = runtime
            .create_task(TaskType::Background, "Task A", EventCorrelation::default(), TaskOwner::User)
            .await;
        let id_b = runtime
            .create_task(TaskType::BrowserAgent, "Task B", EventCorrelation::default(), TaskOwner::User)
            .await;
        let id_c = runtime
            .create_task(TaskType::DevAgent, "Task C", EventCorrelation::default(), TaskOwner::User)
            .await;

        let token_a = runtime.get_cancellation_token(&id_a).await.unwrap();
        let token_b = runtime.get_cancellation_token(&id_b).await.unwrap();
        let token_c = runtime.get_cancellation_token(&id_c).await.unwrap();

        // Start all three concurrently
        assert!(runtime.start_task(&id_a).await.is_ok());
        assert!(runtime.start_task(&id_b).await.is_ok());
        assert!(runtime.start_task(&id_c).await.is_ok());

        // Cancel Task A
        assert!(runtime.cancel_task(&id_a, Some("Abort A".to_string())).await.is_ok());

        // Complete Task B
        assert!(runtime.complete_task(&id_b, "B succeeded").await.is_ok());

        // Fail Task C
        assert!(runtime.fail_task(&id_c, "C crashed").await.is_ok());

        // Verify tokens isolation
        assert!(token_a.is_cancelled());
        assert!(!token_b.is_cancelled());
        assert!(!token_c.is_cancelled());

        // Verify statuses isolation
        assert_eq!(runtime.get_task(&id_a).await.unwrap().status, TaskStatus::Cancelled);
        assert_eq!(runtime.get_task(&id_b).await.unwrap().status, TaskStatus::Completed);
        assert_eq!(runtime.get_task(&id_c).await.unwrap().status, TaskStatus::Failed);

        // Active tasks count should now be 0 since all 3 reached terminal states
        assert_eq!(runtime.list_active_tasks().await.len(), 0);
        assert_eq!(runtime.list_all_tasks().await.len(), 3);
    }
}
