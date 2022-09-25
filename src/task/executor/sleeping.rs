use core::task::{Context, Poll, Waker};

use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use crossbeam_queue::ArrayQueue;

use crate::task::{Task, TaskId};

pub struct SleepingExecutor {
    tasks: BTreeMap<TaskId, (Task, Option<Waker>)>,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl SleepingExecutor {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, (task, None)).is_some() {
            panic!("task with same ID already in tasks");
        }
        self.task_queue.push(task_id).expect("queue full");
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    pub fn run_ready_tasks(&mut self) {
        // Loop over all tasks in the task queue
        while let Some(task_id) = self.task_queue.pop() {
            // Remove the task (and its potential waker) from the task map
            let (task, cached_waker) = match self.tasks.get_mut(&task_id) {
                Some(t) => t,
                None => continue,
            };

            // Create a new task waker if it doesn't exist yet
            let waker = cached_waker
                .get_or_insert_with(|| TaskWaker::waker(task_id, self.task_queue.clone()));

            // Poll the task, removing it from the task map if it's done
            let mut context = Context::from_waker(waker);
            if let Poll::Ready(()) = task.poll(&mut context) {
                self.tasks.remove(&task_id);
            }
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts;

        interrupts::disable();
        if self.task_queue.is_empty() {
            interrupts::enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn waker(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(Self {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
