# Benchmarks

## Table of Contents

- [Overview](#overview)
- [Test Subject](#test-subject)
- [Input Data](#input-data)
- [Reference Setups](#reference-setups)
- [Unit Tests](#unit-tests)
- [Benchmark Results](#benchmark-results)
    - [Entire Text Parsing](#entire-text-parsing)
    - [Keystrokes Reparsing](#keystrokes-reparsing)
    - [Entire Text Input](#entire-text-input)
    - [Keystroke Writes](#keystroke-writes)
    - [Scanner](#scanner)

## Overview

These tests measure the computational performance of the Document and related
objects, the core components of the Lady Deirdre framework, and compare them
with competitor solutions that fully or partially cover individual Lady Deirdre
features.

Overall, Lady Deirdre, as a general-purpose solution, demonstrates acceptable
performance on the test data, comparable to the reference frameworks,
even though the reference solutions usually perform better in their
specialized niches.

Lady Deirdre's non-incremental parsing algorithm performs nearly as fast as Nom,
the non-incremental parser, on the test data of typical size.

Lady Deirdre performs notably faster than Tree-Sitter, the incremental parser,
when parsing the entire file, and is faster than Tree-Sitter on incremental
reparsing of edits in the test data of typical size. However, Tree-Sitter
outperforms Lady Deirdre when managing text edits in files of large size.

It's worth noting that the benchmark tests were conducted on relatively simple
JSON grammar. The performance of real applications may vary depending on
the grammar complexity.

Additionally, each reference solution has its own unique functional features
that cannot be directly compared and that may impact the performance timings.

For example, Nom and Lady Deirdre are recursive-descent parsers, whereas
Tree-Sitter is a recursive-ascent GLR parser. Nom and Tree-Sitter do not
maintain the source code text, whereas Lady Deirdre has inseparable text
storage.

These and other functional differences between frameworks must be taken into
account when comparing the benchmark results.

## Test Subject

The Lady Deirdre's Document is responsible for source code lexical scanning,
syntax parsing, and text storage.

When used in a programming language compiler, it is specifically optimized for
one-time parsing (Immutable Document). When the Document is part of
a language server, it provides write operations to arbitrary fragments of
the source code text (Mutable Document) that instantly synchronize
the underlying lexical and syntax structure with the changes.

Mutable Documents are designed to be fast enough to handle every keystroke event
from the text editor when the end user writes source code in real time.
The Document achieves these features by maintaining an internal cache of
the lexical and syntax structures and by patching these structures during
the reparsing of small code fragments relative to the changes (incremental
reparsing).

Patching instead of recreating the full syntax tree is especially important
for further incremental semantic analysis stages, as the semantic metadata
caches are bound to particular syntax tree nodes.

When the end user writes to a file, the source code of the file is syntactically
broken most of the time. Therefore, the Document's underlying algorithm
is error-tolerant and capable of maintaining the syntax tree of the source code
even with syntax errors.

## Input Data

The benchmark tests attempt to measure individual aspects of the framework using
randomly generated JSON files of variable sizes: "Small File" and "Large File."

Both files contain well-balanced, highly nested, and highly branching JSON
structures with random leaves (strings, numbers, etc.).

Additionally, for each file, there is a sequence of **randomly generated edits**
that mimic real end user keystrokes. There are about 2000-3000 individual write
operations per file, including insertions and deletions grouped together.
In each group, the virtual "user" edits the JSON nodes more or less
consistently. There are about 200-300 such groups per sequence, keeping
the initial file size more or less the same.

![Bench operations](bench-ops.gif)

**Small File** contains about 2,000 lines of code (~64 Kbs). This size is
an upper bound for files typically edited in code editors.

**Large File** contains about 40,000 lines of code (~1 Mb). This is an edge
case that mimics a situation where the user accidentally opens a large file
in the code editor.

## Reference Setups

For reference, I compare Lady Deirdre's benchmark results with benchmarks run on
the same data using well-known frameworks that fully or partially cover
similar features:

- [Nom](https://crates.io/crates/nom) as a reference for
  the non-incremental parser.
- [Tree-Sitter](https://crates.io/crates/tree-sitter) as a reference for
  the incremental parser.
- [Ropey](https://crates.io/crates/ropey) as a reference for the text storage
  with random read/write access.
- [Logos](https://crates.io/crates/logos) as a reference for the lexical
  scanner.

## Unit Tests

The benchmarks' input data, Lady Deirdre setup, and the reference frameworks'
setups are covered by unit tests to ensure that the input data is correct
and that the results of the setups match.

## Benchmark Results

### Entire Text Parsing

Measures non-incremental initial lexical scanning and syntax parsing
performance.
Informally, this covers the case when the end user opens the file in the text
editor or when the compiler loads the file from disk.

The Immutable Document (`Lady Deirdre (immutable)`) performs slightly better
than the Mutable Document (`Lady Deirdre (mutable)`) on both files because
the Immutable Document is specifically optimized for one-time parsing.

`Nom` demonstrates the best parsing performance among the setups, although
it is generally comparable to the Immutable Document, considering that
the Lady Deirdre tests include text storage timings.

`Tree-Sitter` shows the worst results among the setups in these tests.

All four setups are generally acceptable for use in language server
applications.

|                  | `Lady Deirdre (mutable)`  | `Lady Deirdre (immutable)`       | `Nom`                            | `Tree-Sitter`                 |
|:-----------------|:--------------------------|:---------------------------------|:---------------------------------|:------------------------------|
| **`Small File`** | `738.09 us` (✅ **1.00x**) | `537.77 us` (✅ **1.37x faster**) | `450.19 us` (✅ **1.64x faster**) | `2.33 ms` (❌ *3.16x slower*)  |
| **`Large File`** | `20.24 ms` (✅ **1.00x**)  | `14.74 ms` (✅ **1.37x faster**)  | `10.74 ms` (🚀 **1.88x faster**) | `55.70 ms` (❌ *2.75x slower*) |

### Keystrokes Reparsing

Measures incremental lexical and syntax reparsing performance when the end user
enters text into the file.

These tests measure the entire set of edits at once, **excluding the initial
parse time** (measured independently in
the [Entire Text Parsing](#entire-text-parsing)
tests).

To estimate the individual amortized keystroke parse time, you can divide these
timings by ~2000 (the size of the edit set), which results in microseconds
per keystroke and is significantly faster than reparsing the entire file.

This outcome proves that both frameworks are error-resistant incremental
parsers.

`Lady Deirdre` performs better than `Tree-Sitter` on typically small files
(up to 2000 lines of code) when parsing JSON files. However, the performance
degrades when reparsing relatively large files, which is a current shortcoming
of the Lady Deirdre architecture.

Additionally, Tree-Sitter does not have built-in source code text storage,
in contrast to Lady Deirdre's Document. Tree-Sitter requires dedicated text
management (e.g., via Ropey). However, the impact of text maintenance
is probably insignificant in these tests due to
the [Keystroke Writes](#keystroke-writes)
benchmark results. Therefore, the text management timings were excluded from
the `Tree-Sitter` benchmark results.

|                  | `Lady Deirdre`            | `Tree-Sitter`                    |
|:-----------------|:--------------------------|:---------------------------------|
| **`Small File`** | `20.59 ms` (✅ **1.00x**)  | `53.08 ms` (❌ *2.58x slower*)    |
| **`Large File`** | `364.22 ms` (✅ **1.00x**) | `59.73 ms` (🚀 **6.10x faster**) |

### Entire Text Input

Measures source code text loading performance.

Even though the lexical scanner is inseparable from Lady Deirdre's Document
object, these tests attempt to isolate text loading time by replacing
the JSON scanner with a simple text line scanner and turning off
the syntax parser.

Ropey, the mutable strings framework, demonstrates notably better performance
than Lady Deirdre in all tests. The immutable setup of Lady Deirdre performs
slightly better than the mutable setup because the immutable Document
was designed for one-time loading.

|                  | `Lady Deirdre (immutable)` | `Lady Deirdre (mutable)`       | `Ropey`                           |
|:-----------------|:---------------------------|:-------------------------------|:----------------------------------|
| **`Small File`** | `201.63 us` (✅ **1.00x**)  | `233.79 us` (❌ *1.16x slower*) | `28.09 us` (🚀 **7.18x faster**)  |
| **`Large File`** | `6.10 ms` (✅ **1.00x**)    | `7.03 ms` (❌ *1.15x slower*)   | `937.65 us` (🚀 **6.50x faster**) |

### Keystroke Writes

Measures source code text mutation performance.

Tests the "write" function performance of Lady Deirdre's mutable Document on
a set of edits, with the syntax parser turned off and the JSON lexical scanner
replaced with a simple text line scanner.

Compares benchmark results with the performance of Ropey's "insert" and "remove"
functions on the same set.

The initial text loading timings are excluded from these tests (they are
separately measured in the [Entire Text Input](#entire-text-input) tests).

|                  | `Lady Deirdre`           | `Ropey`                            |
|:-----------------|:-------------------------|:-----------------------------------|
| **`Small File`** | `1.53 ms` (✅ **1.00x**)  | `408.27 us` (🚀 **3.75x faster**)  |
| **`Large File`** | `11.62 ms` (✅ **1.00x**) | `545.53 us` (🚀 **21.30x faster**) |

### Scanner

Measures performance of the generated JSON lexical scanners.

The `Lady Deirdre` tests do not use the Document object; instead, they run
the generated lexer that only scans the input text without storing
the output tokens. The `Logos` tests, in turn, perform the same task but use
the scanner generated by the Logos framework.

Logos performs better in all tests; however, both frameworks demonstrate
generally acceptable results.

|                  | `Lady Deirdre`            | `Logos`                          |
|:-----------------|:--------------------------|:---------------------------------|
| **`Small File`** | `191.41 us` (✅ **1.00x**) | `86.25 us` (🚀 **2.22x faster**) |
| **`Large File`** | `5.64 ms` (✅ **1.00x**)   | `2.28 ms` (🚀 **2.47x faster**)  |

---
Made with [criterion-table](https://github.com/nu11ptr/criterion-table)

