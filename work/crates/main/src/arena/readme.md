# Versioned arena memory management.

This module contains a set of collections to organize versioned arena memory
management, and an interface to weakly refer items inside this memory.

A long-lived, but frequently updated set of interconnected data can be stored in
a common Container with globally unique identifier [Id] to distinguish between
the Container instances. This container, in turn, can be stored in a static
memory, or in some other long-lived easy to access place.

Inside this Container you can use [Repository] and [Sequence] instances to
store actual mutable and immutable data items accordingly.

Finally, you can implement a domain-specific reference object for your end
users to access data inside this Container. This object will use base
[Ref] object under the hood to refer data inside underlying Repository and
Sequence collections.

As a rule of thumb you can implement an [Identifiable] trait for your Container,
so it would be easier for your users to distinct between Container instances.

```rust
use lady_deirdre::arena::{Id, Identifiable, Repo, Entry};

pub struct IntStorage {
    id: Id,
    inner: Repo<usize>,
}

impl Identifiable for IntStorage {
    fn id(&self) -> Id { self.id }
}

impl IntStorage {
    pub fn new() -> Self {
        Self {
            // Id::new() returns a globally unique value. 
            id: Id::new(),
            // Alternatively use Repo::with_capacity(...).
            inner: Repo::default(),
        }
    }

    pub fn add(&mut self, item: usize) -> IntRef {
        // This is an "O(1)" time operation.
        //
        // Returning value is always uniquely identifies the Item across other
        // "self.set" items.
        let entry = self.inner.insert(item);

        // This is a "weak" reference to a corresponding Item stored in the
        // `IntStorage` collection.
        //
        // If the user lost this reference, there is no way to access this
        // Item anymore. However, the lifetime of the stored Item is independent
        // from the reference lifetime(reference counting is not involved here).
        // Moreover, the returned weak references are not guaranteed to be
        // valid(as they are "weak").
        //
        // It is up to your system design(to you) to decide about the memory
        // cleanup approaches.
        IntRef {
            id: self.id,
            entry,
        }
    }
}

// It is cheap and safe to copy both Id and Ref. And this object is Send+Sync
// by default.
#[derive(Clone, Copy)]
pub struct IntRef {
    id: Id,
    entry: Entry,
}

impl Identifiable for IntRef {
    fn id(&self) -> Id { self.id }
}

impl IntRef {
    // The end user dereferences this weak reference providing an IntStorage
    // instance the referred Item instance belongs to.
    //
    // If the end users have a set of such IntStorage instances, they can
    // lookup for corresponding instance using e.g. IntRef::id() value. 
    pub fn deref<'a>(&self, storage: &'a IntStorage) -> Option<&'a usize> {
        if self.id != storage.id {
            // The end user provided incorrect IntStorage instance, dereference
            // has failed.
            return None;
        }

        // Returns "Some" if referred Item still exists in this Repo,
        // otherwise returns "None"(IntRef weak reference considered obsolete).
        storage.inner.get(&self.entry)
    }

    // Returns removed Item from provided IntStorage instance if the referred
    // Item exists in this storage.
    //
    // Otherwise returns None (the IntRef instance is obsolete or invalid).
    pub fn remove(&self, storage: &mut IntStorage) -> Option<usize> {
        if self.id != storage.id {
            // The end user provided incorrect IntRef instance.
            return None;
        }

        storage.inner.remove(&self.entry)
    }
}
```

This is a common pattern used across the entire crate API. For example,
[Document](crate::Document) uses [Repository] under the hood to resolve
[TokenRef](crate::lexis::TokenRef) weak references to the stored Tokens of
the Source Code.
