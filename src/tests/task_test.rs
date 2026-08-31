/// 单元测试
#[cfg(test)]
mod tests {
    use crate::task::*;
    use chrono::{DateTime, Days, Utc};
    use std::ops::Add;

    #[test]
    fn test_task_new() {
        let id: usize = 1001;
        let content: String = String::from("test_content1");
        let deadline1: DateTime<Utc> = "2000-01-01T12:00:00+00:00".parse().unwrap();
        let description = Some(String::from("test_description1"));
        let priority = Priority::default();
        let mut task: Task = Task::new(id, content, description, Some(deadline1), priority);

        assert_eq!(task.id(), 1001);
        assert_eq!(task.content(), "test_content1".to_string());
        assert_eq!(task.description(), Some("test_description1"));
        assert_eq!(task.priority(), Priority::Low);
        assert!(!task.is_complete());

        let expected = "2000-01-01T12:00:00+00:00".parse().unwrap();
        assert_eq!(task.deadline(), Some(expected));

        task.set_content("test_content2".to_string());
        assert_eq!(task.content(), "test_content2".to_string());
        task.set_description(Some("test_description2".to_string()));
        assert_eq!(task.description(), Some("test_description2"));
        task.set_description(None);
        assert!(task.description().is_none());
        task.complete();
        assert!(task.is_complete());
        task.incomplete();
        assert!(!task.is_complete());
        task.set_priority(Priority::High);
        assert_eq!(task.priority(), Priority::High);

        let deadline2: DateTime<Utc> = "2000-01-02T12:00:00+00:00".parse().unwrap();
        task.set_deadline(Some(deadline2));
        let expected = "2000-01-02T12:00:00+00:00".parse().unwrap();
        assert_eq!(task.deadline(), Some(expected));
        task.set_deadline(None);
        assert!(task.deadline().is_none());
    }

    #[test]
    fn test_task_is_overdue() {
        let id: usize = 1001;
        let content: String = String::from("test_content1");
        let deadline1: DateTime<Utc> = "2000-01-01T12:00:00+00:00".parse().unwrap();
        let description = Some(String::from("test_description1"));
        let priority = Priority::default();
        let mut task: Task = Task::new(id, content, description, Some(deadline1), priority);
        assert!(task.is_overdue(Utc::now()));

        task.complete();
        assert!(!task.is_overdue(Utc::now()));

        task.incomplete();
        assert!(task.is_overdue(Utc::now()));
        task.set_deadline(None);
        assert!(!task.is_overdue(Utc::now()));

        let deadline2: DateTime<Utc> = Utc::now().add(Days::new(1));
        task.set_deadline(Some(deadline2));
        assert!(!task.is_overdue(Utc::now()));
    }
}
