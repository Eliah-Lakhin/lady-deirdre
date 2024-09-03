<!------------------------------------------------------------------------------
  This file is part of "Lady Deirdre", a compiler front-end foundation
  technology.

  This work is proprietary software with source-available code.

  To copy, use, distribute, or contribute to this work, you must agree to
  the terms of the General License Agreement:

  https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md

  The agreement grants a Basic Commercial License, allowing you to use
  this work in non-commercial and limited commercial products with a total
  gross revenue cap. To remove this commercial limit for one of your
  products, you must acquire a Full Commercial License.

  If you contribute to the source code, documentation, or related materials,
  you must grant me an exclusive license to these contributions.
  Contributions are governed by the "Contributions" section of the General
  License Agreement.

  Copying the work in parts is strictly forbidden, except as permitted
  under the General License Agreement.

  If you do not or cannot agree to the terms of this Agreement,
  do not use this work.

  This work is provided "as is", without any warranties, express or implied,
  except where such disclaimers are legally invalid.

  Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).
  All rights reserved.
------------------------------------------------------------------------------->

# Configuration Issues

Many functions in the semantic analysis framework API can return
an [AnalysisError](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/analysis/enum.AnalysisError.html),
representing either a normal result (e.g.,
an [Interrupted](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/analysis/enum.AnalysisError.html#variant.Interrupted)
error) or an abnormal error indicating a configuration or usage issue with the
framework.

For example,
the [write_to_doc](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/analysis/trait.MutationAccess.html#method.write_to_doc)
function of the mutation task can return
a [MissingDocument](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/analysis/enum.AnalysisError.html#variant.MissingDocument)
error if you specify a document ID that does not exist in the Analyzer (e.g., if
the document was previously removed from the Analyzer).

The API documentation for framework functions typically describes the types of
errors that a function can return. Depending on the situation, you may handle
certain errors manually. However, as a fallback, it is recommended to return
normal errors from functions that
return [AnalysisResult](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/analysis/type.AnalysisResult.html)
and to panic immediately if an abnormal error occurs. This convention helps
identify configuration or usage issues more quickly.

Canonical compilers written with Lady Deirdre should be designed to be
infallible. If you receive an abnormal error from a framework function, it
likely indicates a bug in your program's code that needs to be fixed.

In particular, the computable functions of
the [Chain Analysis](https://github.com/Eliah-Lakhin/lady-deirdre/blob/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/chain_analysis/semantics.rs#L337)
example use
the [unwrap_abnormal](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/analysis/type.AnalysisResult.html#method.unwrap_abnormal)
helper function to filter out normal errors from abnormal ones, panicking if an
abnormal error is encountered.

```rust,noplayground
impl SharedComputable for BlockNamespaceMap {
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

        // The `Semantics::get` function returns a reference to the node's
        // semantics. However, in theory, it could also return
        // an `UninitSemantics` error if the semantics of the node were not
        // properly initialized for some obscure reason.
        //
        // In such a case, the `unwrap_abnormal` function will panic accordingly.
        let block_semantics = semantics.get().unwrap_abnormal()?;

        Ok(block_semantics
            .analysis
            .read(context)
            // If the `Attr::read` function returns an Interrupted error, this
            // error will be propagated from this computable function as well.
            //
            // However, if the function returns any other type of error,
            // considered as abnormal, the `unwrap_abnormal` function will
            // also panic accordingly.
            .unwrap_abnormal()?
            .blocks
            .clone())
    }
}
```

Additionally, it is recommended to log every computable function at the
beginning of its implementation. This practice aids in swiftly identifying
various issues in the attributes logic by examining the log trace.

In the provided snippet, the `log_attr` function generates a debug message for
the logger regarding the computable function about to be executed, along with
the syntax tree node snippet on which this attribute is being computed. This
function is implemented within the Chain Analysis example's codebase. Lady
Deirdre does not include built-in functionality for logging computable
functions, as it does not have a built-in logger and assumes that logging
infrastructure is implementation-specific.

## Cycles Detection

The absence of cycles in the semantic graph is a framework requirement that
users need to implement manually.

Graph cycles share similarities with unbounded infinite recursion accidentally
introduced into the source code. Lady Deirdre cannot proactively check the graph
structure because it evolves at runtime based on custom logic within computable
functions.

There are two primary methods for detecting accidentally introduced cycles.
Firstly, logging computable functions helps establish how attributes refer to
each other during execution via the log trace.

Secondly, a hard timeout limits computable function execution. Typically, if the
semantic model of the language is granular, computable functions complete
execution within a few milliseconds, even on low-end CPUs. By default, the
Analyzer sets the timeout limit to a few seconds[^timeoutlimit]. If a computable
function exceeds this limit, it indicates a potential issue in the semantics
design, and the corresponding analysis function (
e.g., [Attr::snapshot](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/analysis/struct.Attr.html#method.snapshot))
yields
a [Timeout](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/analysis/enum.AnalysisError.html#variant.Timeout)
error.

This mechanism is also useful for detecting cycles. When the semantic graph
validator encounters a cyclic reference between attributes, it deadlocks the
validation procedure. However, due to the timeout limit, the validator
eventually unblocks with a *Timeout* error.

During debugging (when the `debug_assertions` feature flag is enabled), the
*Timeout* error is considered abnormal. Debug builds aim to detect attributes
with long execution times and cycles in the semantic graph as early as possible
for debugging purposes. However, in production builds, *Timeout* errors are
treated as normal errors, assumed to be caused by edge cases in the project's
source code compilation, and thus handled without panic[^timoutpanic].

[^timeoutlimit]: You can configure this limit via
the [AnalyzerConfig](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/analysis/struct.AnalyzerConfig.html)
object, which you pass to the Analyzer's constructor.

[^timoutpanic]: The user of the code editor's extension would prefer the
extension to gracefully ignore specific edge cases that the plugin is unable to
handle, rather than causing the entire plugin to terminate abruptly.
