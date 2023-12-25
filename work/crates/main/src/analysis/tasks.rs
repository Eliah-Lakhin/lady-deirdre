////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" Work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This Work is a proprietary software with source available code.            //
//                                                                            //
// To copy, use, distribute, and contribute into this Work you must agree to  //
// the terms of the End User License Agreement:                               //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The Agreement let you use this Work in commercial and non-commercial       //
// purposes. Commercial use of the Work is free of charge to start,           //
// but the Agreement obligates you to pay me royalties                        //
// under certain conditions.                                                  //
//                                                                            //
// If you want to contribute into the source code of this Work,               //
// the Agreement obligates you to assign me all exclusive rights to           //
// the Derivative Work or contribution made by you                            //
// (this includes GitHub forks and pull requests to my repository).           //
//                                                                            //
// The Agreement does not limit rights of the third party software developers //
// as long as the third party software uses public API of this Work only,     //
// and the third party software does not incorporate or distribute            //
// this Work directly.                                                        //
//                                                                            //
// AS FAR AS THE LAW ALLOWS, THIS SOFTWARE COMES AS IS, WITHOUT ANY WARRANTY  //
// OR CONDITION, AND I WILL NOT BE LIABLE TO ANYONE FOR ANY DAMAGES           //
// RELATED TO THIS SOFTWARE, UNDER ANY KIND OF LEGAL CLAIM.                   //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this Work.                                                      //
//                                                                            //
// Copyright (c) 2022 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use crate::{
    analysis::{AnalysisError, AnalysisResult, Analyzer, Grammar},
    report::{debug_unreachable, system_panic},
    std::*,
    sync::{Latch, SyncBuildHasher},
};

pub const TASKS_ANALYSIS: u8 = 1u8 << 0;
pub const TASKS_EXCLUSIVE: u8 = 1u8 << 1;
pub const TASKS_MUTATION: u8 = 1u8 << 2;
pub const TASKS_ALL: u8 = TASKS_ANALYSIS | TASKS_EXCLUSIVE | TASKS_MUTATION;

const TASKS_CAPACITY: usize = 10;

pub trait AbstractTask<'a, N: Grammar, S: SyncBuildHasher> {
    fn analyzer(&self) -> &'a Analyzer<N, S>;

    fn handle(&self) -> &'a Latch;

    #[inline(always)]
    fn proceed(&self) -> AnalysisResult<()> {
        if self.handle().get_relaxed() {
            return Err(AnalysisError::Interrupted);
        }

        Ok(())
    }
}

pub(super) enum Exclusivity {
    NonExclusive,
    Exclusive,
}

pub(super) struct TaskManager<S> {
    state: Mutex<State<S>>,
    notifiers: Notifiers,
}

impl<S: SyncBuildHasher> TaskManager<S> {
    #[inline(always)]
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(State {
                stage: Stage::Mutation {
                    mutation_tasks: new_tasks_set(),
                },
                pending: Pending {
                    analysis: 0,
                    mutations: 0,
                    exclusive: 0,
                },
            }),
            notifiers: Notifiers {
                ready_for_analysis: Condvar::new(),
                ready_for_mutation: Condvar::new(),
                ready_for_exclusive: Condvar::new(),
            },
        }
    }

    #[inline(always)]
    pub(super) fn interrupt(&self, tasks_mask: u8) {
        self.lock_state().interrupt(tasks_mask);
    }

    #[inline(always)]
    pub(super) fn acquire_analysis(&self, handle: &Latch, lock: bool) -> AnalysisResult<()> {
        let mut state_guard = self.lock_state();

        state_guard.pending.analysis += 1;

        loop {
            self.transit(&mut state_guard);

            let state = state_guard.deref_mut();

            let Stage::Analysis { analysis_tasks } = &mut state.stage else {
                if !lock {
                    state.pending.analysis -= 1;

                    return Err(AnalysisError::Interrupted);
                }

                state_guard = self
                    .notifiers
                    .ready_for_analysis
                    .wait(state_guard)
                    .unwrap_or_else(|poison| poison.into_inner());

                continue;
            };

            state.pending.analysis -= 1;

            if !analysis_tasks.insert(handle.clone()) {
                return Err(AnalysisError::DuplicateHandle);
            }

            return Ok(());
        }
    }

    #[inline(always)]
    pub(super) fn acquire_exclusive(&self, handle: &Latch, lock: bool) -> AnalysisResult<()> {
        let mut state_guard = self.lock_state();

        state_guard.pending.exclusive += 1;

        loop {
            self.transit(&mut state_guard);

            let state = state_guard.deref_mut();

            let Stage::Exclusive { exclusive_task } = &mut state.stage else {
                if !lock {
                    state.pending.exclusive -= 1;

                    return Err(AnalysisError::Interrupted);
                }

                state_guard = self
                    .notifiers
                    .ready_for_analysis
                    .wait(state_guard)
                    .unwrap_or_else(|poison| poison.into_inner());

                continue;
            };

            state.pending.exclusive -= 1;

            return match exclusive_task {
                Some(current) if current == handle => Err(AnalysisError::DuplicateHandle),

                None => {
                    *exclusive_task = Some(handle.clone());

                    Ok(())
                }

                _ => continue,
            };
        }
    }

    #[inline(always)]
    pub(super) fn acquire_mutation(&self, handle: &Latch, lock: bool) -> AnalysisResult<()> {
        let mut state_guard = self.lock_state();

        state_guard.pending.mutations += 1;

        loop {
            self.transit(&mut state_guard);

            let state = state_guard.deref_mut();

            let Stage::Mutation { mutation_tasks } = &mut state.stage else {
                if !lock {
                    state.pending.mutations -= 1;

                    return Err(AnalysisError::Interrupted);
                }

                state_guard = self
                    .notifiers
                    .ready_for_analysis
                    .wait(state_guard)
                    .unwrap_or_else(|poison| poison.into_inner());

                continue;
            };

            state.pending.mutations -= 1;

            if !mutation_tasks.insert(handle.clone()) {
                return Err(AnalysisError::DuplicateHandle);
            }

            return Ok(());
        }
    }

    #[inline(always)]
    pub(super) fn release_analysis(&self, handle: &Latch) {
        let mut state_guard = self.lock_state();

        match &mut state_guard.stage {
            Stage::Analysis { analysis_tasks } => {
                if !analysis_tasks.remove(handle) {
                    system_panic!("Missing handle.");
                }

                if analysis_tasks.is_empty() {
                    analysis_tasks.shrink_to(TASKS_CAPACITY);
                }
            }

            Stage::Interruption { pending } => {
                *pending = match pending.checked_sub(1) {
                    Some(counter) => counter,
                    None => {
                        system_panic!("Interruption counter mismatch.");

                        // Safety: system_panic interrupts thread.
                        unsafe { debug_unreachable!("Life after panic.") }
                    }
                };

                if *pending > 0 {
                    return;
                }
            }

            _ => system_panic!("Stage mismatch."),
        }

        self.transit(&mut state_guard);
    }

    #[inline(always)]
    pub(super) fn release_exclusive(&self, handle: &Latch) {
        let mut state_guard = self.lock_state();

        match &mut state_guard.stage {
            Stage::Exclusive { exclusive_task } => {
                let exclusive_task = take(exclusive_task);

                if exclusive_task.as_ref() != Some(handle) {
                    system_panic!("Missing handle.");
                }
            }

            Stage::Interruption { pending } => {
                *pending = match pending.checked_sub(1) {
                    Some(counter) => counter,
                    None => {
                        system_panic!("Interruption counter mismatch.");

                        // Safety: system_panic interrupts thread.
                        unsafe { debug_unreachable!("Life after panic.") }
                    }
                };

                if *pending > 0 {
                    return;
                }
            }

            _ => system_panic!("Stage mismatch."),
        }

        self.transit(&mut state_guard);
    }

    #[inline(always)]
    pub(super) fn release_mutation(&self, handle: &Latch) {
        let mut state_guard = self.lock_state();

        match &mut state_guard.stage {
            Stage::Mutation { mutation_tasks } => {
                if !mutation_tasks.remove(handle) {
                    system_panic!("Missing handle.");
                }

                if mutation_tasks.is_empty() {
                    mutation_tasks.shrink_to(TASKS_CAPACITY);
                }
            }

            Stage::Interruption { pending } => {
                *pending = match pending.checked_sub(1) {
                    Some(counter) => counter,
                    None => {
                        system_panic!("Interruption counter mismatch.");

                        // Safety: system_panic interrupts thread.
                        unsafe { debug_unreachable!("Life after panic.") }
                    }
                };

                if *pending > 0 {
                    return;
                }
            }

            _ => system_panic!("Stage mismatch."),
        }

        self.transit(&mut state_guard);
    }

    #[inline(always)]
    fn transit(&self, state_guard: &mut MutexGuard<State<S>>) {
        state_guard.interrupt(0);
        state_guard.transit(&self.notifiers);
    }

    #[inline(always)]
    fn lock_state(&self) -> MutexGuard<State<S>> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

struct State<S> {
    stage: Stage<S>,
    pending: Pending,
}

impl<S: SyncBuildHasher> State<S> {
    #[inline(always)]
    fn interrupt(&mut self, force_mask: u8) {
        match &mut self.stage {
            Stage::Analysis { analysis_tasks } => {
                if force_mask & TASKS_ANALYSIS == 0 {
                    if analysis_tasks.is_empty() {
                        return;
                    }

                    if self.pending.mutations == 0 && self.pending.exclusive == 0 {
                        return;
                    }
                }

                let analysis_tasks = take(analysis_tasks);

                self.stage = Stage::Interruption {
                    pending: analysis_tasks.len(),
                };

                for handle in analysis_tasks {
                    handle.set();
                }
            }

            Stage::Exclusive { exclusive_task } => {
                if force_mask & TASKS_EXCLUSIVE == 0 {
                    if exclusive_task.is_none() {
                        return;
                    }

                    if self.pending.mutations == 0 {
                        return;
                    }
                }

                let Some(handle) = take(exclusive_task) else {
                    self.stage = Stage::Interruption { pending: 0 };
                    return;
                };

                self.stage = Stage::Interruption { pending: 1 };

                handle.set();
            }

            Stage::Mutation { mutation_tasks } => {
                if force_mask & TASKS_MUTATION == 0 {
                    return;
                }

                let mutation_tasks = take(mutation_tasks);

                self.stage = Stage::Interruption {
                    pending: mutation_tasks.len(),
                };

                for handle in mutation_tasks {
                    handle.set();
                }
            }

            Stage::Interruption { .. } => {}
        }
    }

    #[inline(always)]
    fn transit(&mut self, notifiers: &Notifiers) {
        if self.stage.is_locked() {
            return;
        }

        if self.pending.mutations > 0 {
            if let Stage::Mutation { .. } = &self.stage {
                return;
            }

            self.stage = Stage::Mutation {
                mutation_tasks: new_tasks_set(),
            };

            notifiers.ready_for_mutation.notify_all();

            return;
        }

        if self.pending.exclusive > 0 {
            if let Stage::Exclusive { .. } = &self.stage {
                return;
            }

            self.stage = Stage::Exclusive {
                exclusive_task: None,
            };

            notifiers.ready_for_exclusive.notify_one();

            return;
        }

        if self.pending.analysis > 0 {
            if let Stage::Analysis { .. } = &self.stage {
                return;
            }

            self.stage = Stage::Analysis {
                analysis_tasks: new_tasks_set(),
            };

            notifiers.ready_for_analysis.notify_all();

            return;
        }
    }
}

enum Stage<S> {
    Analysis { analysis_tasks: HashSet<Latch, S> },
    Exclusive { exclusive_task: Option<Latch> },
    Mutation { mutation_tasks: HashSet<Latch, S> },
    Interruption { pending: usize },
}

impl<S: SyncBuildHasher> Stage<S> {
    #[inline(always)]
    fn is_locked(&self) -> bool {
        match self {
            Stage::Analysis { analysis_tasks } => !analysis_tasks.is_empty(),
            Stage::Exclusive { exclusive_task } => exclusive_task.is_some(),
            Stage::Mutation { mutation_tasks } => !mutation_tasks.is_empty(),
            Stage::Interruption { pending } => *pending > 0,
        }
    }
}

struct Notifiers {
    ready_for_analysis: Condvar,
    ready_for_exclusive: Condvar,
    ready_for_mutation: Condvar,
}

struct Pending {
    analysis: usize,
    exclusive: usize,
    mutations: usize,
}

#[inline(always)]
fn new_tasks_set<S: SyncBuildHasher>() -> HashSet<Latch, S> {
    HashSet::with_capacity_and_hasher(TASKS_CAPACITY, S::default())
}
