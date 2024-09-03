////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    fmt::{Debug, Formatter},
    sync::{Condvar, Mutex, MutexGuard},
};

use crate::{
    analysis::{AnalysisError, AnalysisResult},
    report::{ld_assert, ld_unreachable},
    sync::{Shared, SyncBuildHasher, Trigger},
};

const TASKS_CAPACITY: usize = 10;

/// An object that signals a task worker to finish its job.
///
/// In Lady Deirdre, task jobs are subject for graceful shutdown.
///
/// Each task object provides access to the TaskHandle. The analyzer's functions
/// associated with this task and the user's code assume to examine the
/// [TaskHandle::is_triggered] value periodically to determine if the job needs
/// to be finished earlier. If the function returns true, the worker should
/// finish its job as soon as possible and drop the task.
///
/// Another party of the compiler execution, such as the thread that spawns the
/// worker's sub-thread, and the [Analyzer](crate::analysis::Analyzer) object
/// itself, that have a clone of the TaskHandle object, may trigger spawned
/// worker's job interruption by calling a [TaskHandle::trigger] function that
/// sets the inner flag of the TaskHandle to true.
///
/// The [TriggerHandle] object provides default implementation of TaskHandle
/// backed by a single [Trigger] object. However, in the end compiler
/// architecture, you can implement your own type of TaskHandle with more
/// complex triggering logic.
pub trait TaskHandle: Default + Clone + Send + Sync + 'static {
    /// Returns true if the task's worker should finish its job as soon as
    /// possible and drop the corresponding task object.
    fn is_triggered(&self) -> bool;

    /// Signals the task's worker to finish its job as soon as possible and to
    /// drop the corresponding task.
    ///
    /// The function sets this TaskHandle's state and all of its clones states
    /// to "triggered" such that their [is_triggered](Self::is_triggered)
    /// function would return true.
    ///
    /// Once the trigger function is called, the TaskHandle triggering state
    /// cannot be unset.
    fn trigger(&self);
}

/// A default implementation of the [TaskHandle] backed by the [Trigger] object.
#[derive(Default, PartialEq, Eq, Hash, Clone)]
pub struct TriggerHandle(
    /// An inner state of the handle.
    pub Trigger,
);

impl Debug for TriggerHandle {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0.is_active() {
            true => formatter.write_str("TriggerHandle(active)"),
            false => formatter.write_str("TriggerHandle(inactive)"),
        }
    }
}

impl TaskHandle for TriggerHandle {
    #[inline(always)]
    fn is_triggered(&self) -> bool {
        self.0.is_active()
    }

    #[inline(always)]
    fn trigger(&self) {
        self.0.activate();
    }
}

impl TriggerHandle {
    /// Creates a new task handle in "untriggered" state.
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }
}

/// A priority of the task.
///
/// The [analyzer](crate::analysis::Analyzer)'s task manager attempts to
/// grant access to the tasks with higher priority value earlier than the tasks
/// with lower priority.
pub type TaskPriority = u16;

pub(super) type TaskId = u64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TaskKind {
    Analysis,
    Mutation,
    Exclusive,
}

pub(super) struct TaskManager<H, S> {
    state: Mutex<ManagerState<H, S>>,
}

impl<H: TaskHandle, S: SyncBuildHasher> TaskManager<H, S> {
    #[inline(always)]
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(ManagerState {
                next_task_id: 0,
                cancel_threshold: 0,
                active_mode: None,
                active_tasks: HashMap::with_hasher(S::default()),
                awoke_tasks: HashMap::with_hasher(S::default()),
                sleep_tasks: BinaryHeap::new(),
            }),
        }
    }

    pub(super) fn acquire_task(
        &self,
        kind: TaskKind,
        handle: &H,
        priority: TaskPriority,
        lock: bool,
    ) -> AnalysisResult<TaskId> {
        let mut state = self.lock_state();

        if priority < state.cancel_threshold || handle.is_triggered() {
            return Err(AnalysisError::Interrupted);
        }

        let Some(active_mode) = state.active_mode else {
            state.active_mode = Some(kind);

            let task_id = state.gen_task_id();

            state.insert_active_task(task_id, handle.clone(), priority);

            return Ok(task_id);
        };

        ld_assert!(!state.active_tasks.is_empty(), "Empty active tasks map.");

        let mode_fits = active_mode_fits(active_mode, kind);

        if mode_fits && state.pending_priority() <= priority {
            let task_id = state.gen_task_id();

            state.insert_active_task(task_id, handle.clone(), priority);

            return Ok(task_id);
        }

        if !lock {
            return Err(AnalysisError::Interrupted);
        }

        if !mode_fits {
            state.interrupt_active_tasks(priority);
        }

        let task_id = state.gen_task_id();

        let waker = state.enqueue_task(task_id, kind, priority, handle.clone());

        loop {
            state = waker
                .as_ref()
                .wait(state)
                .unwrap_or_else(|poison| poison.into_inner());

            let Some(wakeup_kind) = state.awoke_tasks.remove(&task_id) else {
                continue;
            };

            if state.awoke_tasks.capacity() > TASKS_CAPACITY {
                state.awoke_tasks.shrink_to(TASKS_CAPACITY);
            }

            return match wakeup_kind {
                WakeupKind::Activate => Ok(task_id),
                WakeupKind::Cancel => Err(AnalysisError::Interrupted),
            };
        }
    }

    pub(super) fn release_task(&self, id: TaskId) {
        let mut state = self.lock_state();

        ld_assert!(state.active_mode.is_some(), "Release in inactive mode.");

        if state.active_tasks.remove(&id).is_none() {
            unsafe { ld_unreachable!("Missing active task.") }
        }

        if !state.active_tasks.is_empty() {
            return;
        }

        if state.active_tasks.capacity() > TASKS_CAPACITY {
            state.active_tasks.shrink_to(TASKS_CAPACITY);
        }

        state.active_mode = None;

        loop {
            let Some(sleep_task) = state.sleep_tasks.pop() else {
                break;
            };

            if sleep_task.is_cancelled(state.cancel_threshold) {
                state.wake_up_task(sleep_task.id, &sleep_task.waker, WakeupKind::Cancel);

                continue;
            }

            let kind = sleep_task.kind;

            state.active_mode = Some(kind);

            state.insert_active_task(sleep_task.id, sleep_task.handle, sleep_task.priority);
            state.wake_up_task(sleep_task.id, &sleep_task.waker, WakeupKind::Activate);

            if kind == TaskKind::Exclusive {
                break;
            }

            loop {
                let Some(top) = state.sleep_tasks.peek() else {
                    break;
                };

                if top.kind != kind {
                    break;
                }

                let Some(sleep_task) = state.sleep_tasks.pop() else {
                    unsafe { ld_unreachable!("Missing sleep task.") }
                };

                if sleep_task.is_cancelled(state.cancel_threshold) {
                    state.wake_up_task(sleep_task.id, &sleep_task.waker, WakeupKind::Cancel);

                    continue;
                }

                state.insert_active_task(sleep_task.id, sleep_task.handle, sleep_task.priority);
                state.wake_up_task(sleep_task.id, &sleep_task.waker, WakeupKind::Activate);
            }

            break;
        }

        if state.sleep_tasks.capacity() > TASKS_CAPACITY {
            state.sleep_tasks.shrink_to(TASKS_CAPACITY);
        }
    }

    pub(super) fn set_access_level(&self, threshold: TaskPriority) {
        let mut state = self.lock_state();

        if state.cancel_threshold > threshold {
            state.cancel_threshold = threshold;
            return;
        }

        state.cancel_threshold = threshold;

        state.interrupt_active_tasks(threshold);
        state.cancel_pending_tasks(threshold);
    }

    pub(super) fn get_access_level(&self) -> TaskPriority {
        let state = self.lock_state();

        state.cancel_threshold
    }

    #[inline(always)]
    fn lock_state(&self) -> MutexGuard<ManagerState<H, S>> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

struct ManagerState<H, S> {
    next_task_id: TaskId,
    cancel_threshold: TaskPriority,
    active_mode: Option<TaskKind>,
    active_tasks: HashMap<TaskId, ActiveTaskInfo<H>, S>,
    awoke_tasks: HashMap<TaskId, WakeupKind, S>,
    sleep_tasks: BinaryHeap<SleepTaskInfo<H>>,
}

impl<H: TaskHandle, S: SyncBuildHasher> ManagerState<H, S> {
    #[inline(always)]
    fn wake_up_task(&mut self, id: TaskId, task_waker: &TaskWaker, wakeup_kind: WakeupKind) {
        if self.awoke_tasks.insert(id, wakeup_kind).is_some() {
            unsafe { ld_unreachable!("Duplicate task id.") }
        }

        task_waker.as_ref().notify_one();
    }

    #[inline(always)]
    fn insert_active_task(&mut self, id: TaskId, handle: H, priority: TaskPriority) {
        let info = ActiveTaskInfo {
            priority,
            shutdown: handle,
        };

        if self.active_tasks.insert(id, info).is_some() {
            unsafe { ld_unreachable!("Duplicate task id.") }
        }
    }

    #[inline(always)]
    fn interrupt_active_tasks(&mut self, threshold: TaskPriority) {
        if threshold == 0 {
            return;
        }

        for task_info in self.active_tasks.values() {
            if task_info.priority < threshold {
                task_info.shutdown.trigger();
            }
        }
    }

    #[inline(always)]
    fn cancel_pending_tasks(&mut self, threshold: TaskPriority) {
        if threshold == 0 {
            return;
        }

        self.sleep_tasks.retain(|sleep_task| {
            if sleep_task.priority > threshold {
                return true;
            }

            if self
                .awoke_tasks
                .insert(sleep_task.id, WakeupKind::Cancel)
                .is_some()
            {
                unsafe { ld_unreachable!("Duplicate task id.") }
            }

            sleep_task.waker.as_ref().notify_one();

            false
        })
    }

    #[inline(always)]
    fn enqueue_task(
        &mut self,
        id: TaskId,
        kind: TaskKind,
        priority: TaskPriority,
        handle: H,
    ) -> TaskWaker {
        let waker = Shared::new(Condvar::new());

        self.sleep_tasks.push(SleepTaskInfo {
            id,
            kind,
            priority,
            handle,
            waker: waker.clone(),
        });

        waker
    }

    #[inline(always)]
    fn pending_priority(&self) -> TaskPriority {
        let Some(peek) = self.sleep_tasks.peek() else {
            return 0;
        };

        peek.priority
    }

    #[inline(always)]
    fn gen_task_id(&mut self) -> TaskId {
        self.next_task_id = match self.next_task_id.checked_add(1) {
            Some(id) => id,
            None => panic!("Too many tasks."),
        };

        self.next_task_id
    }
}

type TaskWaker = Shared<Condvar>;

enum WakeupKind {
    Activate,
    Cancel,
}

struct ActiveTaskInfo<H> {
    priority: TaskPriority,
    shutdown: H,
}

struct SleepTaskInfo<H> {
    id: TaskId,
    kind: TaskKind,
    priority: TaskPriority,
    handle: H,
    waker: TaskWaker,
}

impl<H: TaskHandle> PartialEq for SleepTaskInfo<H> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.priority.eq(&other.priority)
    }
}

impl<H: TaskHandle> Eq for SleepTaskInfo<H> {}

impl<H: TaskHandle> PartialOrd for SleepTaskInfo<H> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<H: TaskHandle> Ord for SleepTaskInfo<H> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl<H: TaskHandle> SleepTaskInfo<H> {
    #[inline(always)]
    fn is_cancelled(&self, cancel_threshold: TaskPriority) -> bool {
        self.priority < cancel_threshold || self.handle.is_triggered()
    }
}

#[inline(always)]
fn active_mode_fits(active_mode: TaskKind, task_kind: TaskKind) -> bool {
    active_mode == task_kind && active_mode != TaskKind::Exclusive
}
