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

# Tree Index

The semantic analysis framework of Lady Deirdre is capable of maintaining
user-defined context-unaware indexes of the document's syntax tree nodes.

Examples of these indices include all methods within the document, all code
blocks, identifiers partitioned by name, and so forth.

Indices serve various purposes. For instance, in the previous chapter, we
discussed document-wide diagnostics, where the root's diagnostic attribute
collects local diagnostics from all scope nodes within the document. To support
this attribute, you can define an index of all scopes within the document. The
attribute will then read this class of nodes inside the computable function to
inspect all their local diagnostic attribute values.

Another example involves highlighting all identifiers within the code related to
a single variable. Typically, within the attributes framework, it's easier to
establish the variable usage node's definition node than the opposite relations.
When the end user clicks on the variable usage symbol in the code editor, the
language client requests from the language server all highlighted spans related
to the symbol on which the user clicks.

Here's how you can fulfill a request for highlighting all identifiers within the
code that relate to a single variable:

1. Traverse the syntax tree to determine the node on which the end user
   clicks[^traverse].
2. Query this node's attribute to retrieve its variable definition node. At this
   point, we discover two spans: the variable usage span where the user's cursor
   is and the variable definition span. However, we don't yet know about other
   variable usage spans within the code that would be useful for the editor's
   user.
3. Query the index of all variable usages within the code with the specific
   name. Some of these will be the identifiers we are looking for, while others
   may be false candidates located outside the variable definition scope.
4. Since false candidates are likely to be a relatively small subset, owing to
   the practice of programmers using distinct variable names, filter out these
   false instances in the set. This can be achieved by querying their definition
   attributes again to determine if they have the same definition node as the
   one discovered in step 2.

[^traverse]: You can utilize depth-first traversal using
the [Document::traverse_tree](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/trait.SyntaxTree.html#method.traverse_tree)
function. By skipping the descent into child nodes with spans that don't cover
the targeted site, the traversal complexity averages to `O(ln(N))`, where N is
the number of nodes in the tree. In other words, traversing will typically be
quite fast.

## Index Setup

To enable the index, you need to specify the nodes classifier using
the `#[classifier(...)]` macro attribute.

```rust,noplayground
#[derive(Node)]
#[classifier(ChainNodeClassifier)]
pub enum ChainNode {
   // ...
}
```

The parameter of this macro attribute is an arbitrary type that implements
the [Classifier](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/trait.Classifier.html)
trait. It denotes classes of nodes, essentially serving as indices, and the
function that partitions requested nodes between these classes.

In
the [Chain Analysis](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/work/crates/examples/src/chain_analysis/semantics.rs#L411)
example, we define just one class for all `ChainNode::Key` nodes within the
syntax tree.

```rust,noplayground
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ChainNodeClass {
    AllKeys, // All keys
}

pub struct ChainNodeClassifier;

impl Classifier for ChainNodeClassifier {
    // Must match the type of the syntax tree node.
    type Node = ChainNode;
    
    // Could be any user-defined type eligible for the HashSet. 
    type Class = ChainNodeClass;

    // Given the Document and the NodeRef that points to the node inside this
    // document, this function should compute a set of classes to which this
    // node belongs, possibly returning an empty set.
    fn classify<S: SyncBuildHasher>(
        doc: &Document<Self::Node>,
        node_ref: &NodeRef,
    ) -> HashSet<Self::Class, S> {
        let mut result = HashSet::with_hasher(S::default());

        let Some(node) = node_ref.deref(doc) else {
            return result;
        };

        match node {
            ChainNode::Key { .. } => {
                let _ = result.insert(ChainNodeClass::AllKeys);
            }

            _ => (),
        }

        result
    }
}
```

Inside the classification function, it's recommended (and necessary) to
dereference the specified node to determine its classes. You can examine its
lexical structure but should avoid inspecting the node's parent and child node
structures, as the classification should be context-unaware. Classes of the
nodes are simple lexical classes.

In the above code, we classify the node by its enum discriminant only. In more
complex setups, you can use the TokenRef references of the node, as these
references are part of the node's lexical structure. For example, we could
partition the Keys by their token strings as well, using the strings as part of
the class.

## Index Maintenance

The Analyzer automatically maintains the index. During the initial parsing of
the newly created document, the Analyzer creates a document index by calling the
above function on each syntax tree node, associating each class with the set of
nodes of this class.

When you edit the document (using
the [write_to_doc](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/trait.MutationAccess.html#method.write_to_doc)
function), the Analyzer incrementally updates this partition based on the
changes in the structure of the syntax tree.

## Index Access

You can query a set of nodes of the document that belong to a specified class
both inside the computable functions of the attributes using
the [read_class](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.AttrContext.html#method.read_class)
function of the `context` variable, and outside using the
[snapshot_class](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/trait.AbstractTask.html#method.snapshot_class)
function of the task object.

Both functions return
a [Shared](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/sync/struct.Shared.html)
set of the NodeRefs that point to the nodes in the document's syntax tree
belonging to the class.

When you query the index from inside of the computable function, the attribute
subscribes to changes in this class. Whenever the returning set changes, this
attribute will be invalidated. Therefore, you can traverse and dereference the
nodes from the returning set, and you can read their semantics too inside the
computable function of any kind of attribute. However, in general, you should
avoid inspecting these node structures more deeply.
