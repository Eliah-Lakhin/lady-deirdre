<!------------------------------------------------------------------------------
  This file is a part of the "Lady Deirdre" work,
  a compiler front-end foundation technology.

  This work is proprietary software with source-available code.

  To copy, use, distribute, and contribute to this work, you must agree to
  the terms of the General License Agreement:

  https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.

  The agreement grants you a Commercial-Limited License that gives you
  the right to use my work in non-commercial and limited commercial products
  with a total gross revenue cap. To remove this commercial limit for one of
  your products, you must acquire an Unrestricted Commercial License.

  If you contribute to the source code, documentation, or related materials
  of this work, you must assign these changes to me. Contributions are
  governed by the "Derivative Work" section of the General License
  Agreement.

  Copying the work in parts is strictly forbidden, except as permitted under
  the terms of the General License Agreement.

  If you do not or cannot agree to the terms of this Agreement,
  do not use this work.

  This work is provided "as is" without any warranties, express or implied,
  except to the extent that such disclaimers are held to be legally invalid.

  Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).
  All rights reserved.
------------------------------------------------------------------------------->

# Tasks Management

Under the hood, the Analyzer maintains a queue of tasks, both activated and
pending.

To clarify, at any given point in time, the Analyzer activates only one type of
simultaneous task objects (and no more than one exclusive task).

Whenever you request a task object of a particular type, the task manager
attempts to activate it immediately according to the current tasks queue. If
activation is not possible, the Analyzer blocks the current thread, enqueues the
request, and unblocks the requester thread once the inactive request in the
queue reaches activation (once all top active task objects in the queue that
block this request will be released by the concurrent threads).

## Graceful Shutdown

The job that your program's thread performs with the task object is subject to
graceful shutdown. For this reason, each task request function of the Analyzer
requires specifying the task handle through which the job could be signaled to
shut down.

```rust,noplayground
let handle = TriggerHandle::new();

let task = analyzer.analyze(&handle, 1).unwrap();

assert!(!handle.is_triggered());
assert!(!task.handle().is_triggered());

// Signals the job for interruption.
handle.trigger();

assert!(handle.is_triggered());
assert!(task.handle().is_triggered());
```

The [TriggerHandle](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.TriggerHandle.html)
is the default implementation of the handle[^customhandle]. This object is
thread-safe and cheap to clone. Once the handle is triggered (via
the [trigger](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/trait.TaskHandle.html#tymethod.trigger)
function), all copies of the instance become triggered, which serves as a marker
for the job thread to gracefully finish its job.

You can create the handle from outside of the worker thread where you are
scheduling the worker's job and pass a copy of the handle to the worker thread.
The worker thread will use it to request the task object from the Analyzer.

The worker thread should periodically check the handle's triggering state. Once
the handle is triggered and if the worker hasn't completed its job yet, the
worker thread should perform the minimal amount of work required to interrupt
its job normally and drop the task object as soon as possible, releasing the
acquired access grant back to the Analyzer.

The Analyzer could also trigger the handle. For example, if the task manager
realizes that some thread has requested a task with higher priority and this
kind of access cannot be granted instantly because there are lesser prioritized
but still active task objects in the queue, the manager would trigger the
handles of these active tasks.

[^customhandle]: Note that Lady Deirdre allows you to create your own task
handle types with more complex triggering logic by implementing
the [TaskHandle](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/trait.TaskHandle.html)
trait on the user-defined type. In this case, you would use this type as the
second generic parameter of
the [Analyzer](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Analyzer.html)
object.

## Tasks Interruption

The Analyzer itself examines the handle during the semantic graph validation
between the attribute validation bounds. If the validation procedure determines
that the handle was triggered in the middle of the analysis, the validator will
leave the semantic graph in a not-yet-completed state (without breaking its
integrity), and it will start returning
the [Interrupted](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/enum.AnalysisError.html#variant.Interrupted)
error from all corresponding functions.

For example,
the [Attr::read](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Attr.html#method.read)
function used inside the computable functions and
the [Attr::snapshot](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Attr.html#method.snapshot)
function used to get a copy of the attribute value from outside both would start
returning the Interrupted error.

Receiving this error signals that the task handle was triggered, indicating that
you are no longer able to query the semantic graph using this task object, and
that you should gracefully finish the worker's job by dropping the task object
as soon as possible.

When you receive this error inside the computable function, you should return
this error as well from the function.

```rust,noplayground
#[derive(Default, Clone, PartialEq, Eq)]
pub struct BlockAssignmentMap {
    pub map: HashMap<NodeRef, Shared<LocalResolution>>,
}

impl SharedComputable for BlockAssignmentMap {
    type Node = ChainNode;

    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr::<Self, H, S>(context)?;

        let block_ref = context.node_ref();
        let doc_read = context.read_doc(block_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let Some(ChainNode::Block { semantics, .. }) = block_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        let block_semantics = semantics.get().unwrap_abnormal()?;

        // The `?` mark would return the Interrupted error from this function if
        // the `read` function was interrupted.
        Ok(block_semantics.analysis.read(context)?.assignments.clone())
    }
}
```

Note that the validator checks interruption events only between computable
function calls. In principle, it is not capable of checking this event during
function execution. To make the trigger handle state examination more granular,
you can manually check its state in long-running computable functions.

For instance, in
the [BlockAnalysis](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/work/crates/examples/src/chain_analysis/semantics.rs#L223)
attribute of the Chain Analysis example, we are checking the interruption state
during the iteration through the assignment statements of the block.

```rust,noplayground
impl Computable for BlockAnalysis {
    type Node = ChainNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        let block_ref = context.node_ref();
        let doc_read = context.read_doc(block_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let mut result = Self::default();

        let Some(ChainNode::Block { statements, .. }) = block_ref.deref(doc) else {
            return Ok(result);
        };

        let mut block_namespace = BlockNamespace::default();

        for st_ref in statements {
            // Returns an Interrupted error if the task handle had been triggered.
            context.proceed()?;

            // ...
        }

        Ok(result)
    }
}
```

## Tasks Priority

The second argument of the task request functions (e.g., *analyze*, *mutate*,
etc.) is a numeric type denoting the task's priority.

The task manager attempts to prioritize tasks with a higher priority number over
tasks with a lower priority number when enqueueing the task object into the task
queue.

## Bulk Interruption

You can specify the minimum tasks priority level allowed in the Analyzer by
calling
the [Analyzer::set_access_level](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Analyzer.html#method.set_access_level)
function and specifying the priority threshold.

Calling this function will interrupt all currently active tasks with a priority
lower than the threshold.

Non-active pending tasks with a priority lower than the threshold will be
removed from the task queue, and the corresponding requester threads will be
unblocked, immediately
receiving [Interrupted](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/enum.AnalysisError.html#variant.Interrupted)
errors.

All future task requests with a lower priority than the current threshold will
also receive Interrupted errors.

This function is particularly useful for shutting down the entire compiler
gracefully. By specifying the maximum threshold value, you can enforce all tasks
of all kinds to shut down gracefully, preventing access from being granted to
any new incoming task requests.
